use crate::model::ip_detail::IpDetail;
use crate::scrapy::crawling;
use crate::scrapy::crawling_rule::CrawlingRule;
use crate::service::ip_cache;
use crate::service::pool::Pool;
use log::{error, info};
use redis::aio::ConnectionManager;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub fn start(redis: Arc<Mutex<ConnectionManager>>, pool: Pool) {
    let r1 = redis.clone();
    let p1 = pool.clone();
    tokio::spawn(async move {
        loop {
            if let Err(e) = crawl_task(r1.clone(), p1.clone()).await {
                error!("crawl_task exited: {}, restart in 60s", e);
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });

    tokio::spawn(async move {
        loop {
            if let Err(e) = verify_task(redis.clone(), pool.clone()).await {
                error!("verify_task exited: {}, restart in 60s", e);
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
}

async fn crawl_task(redis: Arc<Mutex<ConnectionManager>>, pool: Pool) -> Result<(), String> {
    loop {
        info!("crawl task start");

        let json = include_bytes!("../../resource/crawling_rules.json");
        let rules: Vec<CrawlingRule> = serde_json::from_slice(json)
            .map_err(|e| format!("parse crawling_rules.json error: {}", e))?;

        for rule in rules {
            let ips = crawling::crawling(&rule).await;
            info!("rule {} crawled {} ips", &rule.name, ips.len());

            let mut handles = Vec::new();
            for ip in ips {
                let redis = redis.clone();
                handles.push(pool.spawn(async move {
                    if ip_cache::check_ip(&ip).await {
                        ip_cache::ip_in_redis(redis.clone(), ip).await;
                    }
                }));
            }
            for h in handles {
                if let Err(e) = h.await {
                    error!("crawl verify subtask failed: {}", e);
                }
            }
        }

        info!("crawl task done, sleep 12h");
        tokio::time::sleep(Duration::from_secs(12 * 3600)).await;
    }
}

async fn verify_task(redis: Arc<Mutex<ConnectionManager>>, pool: Pool) -> Result<(), String> {
    loop {
        tokio::time::sleep(Duration::from_secs(10 * 60)).await;
        info!("verify task start");

        let ips = ip_cache::get_all_ips(redis.clone()).await;
        info!("got {} ips to verify", ips.len());

        let mut handles = Vec::new();
        for ip in ips {
            let redis = redis.clone();
            handles.push(pool.spawn(async move {
                if ip_cache::check_ip(&ip).await {
                    let ok = IpDetail {
                        verify_count: ip.verify_count + 1,
                        live_time: ip.live_time + 10 * 60 * 1000,
                        ..ip
                    };
                    ip_cache::ip_in_redis(redis.clone(), ok).await;
                } else {
                    ip_cache::ip_in_redis(redis.clone(), IpDetail::died(ip)).await;
                }
            }));
        }
        for h in handles {
            if let Err(e) = h.await {
                error!("verify subtask failed: {}", e);
            }
        }

        info!("verify task done");
    }
}
