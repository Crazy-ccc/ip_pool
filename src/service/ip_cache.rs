use crate::model::ip_detail::IpDetail;
use crate::{AppState, Resp};
use actix_web::{Responder, Scope, web};
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, RedisResult};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub fn service() -> Scope {
    web::scope("/cache")
        .route("/ip", web::get().to(get_ip))
        .route("/count", web::get().to(get_count))
}

async fn get_ip(
    query: web::Query<HashMap<String, String>>,
    state: web::Data<AppState>,
) -> impl Responder {
    let query = query.into_inner();

    let mut search = "ip_cache::*".to_string();
    // protocol_type(可选)：代理协议，http，socks4，socks5，https
    if let Some(protocol_type) = query.get("protocol_type") {
        search = format!("ip_cache::{}::*", protocol_type);
    }
    // 1→高匿，2→普匿，3→匿名，4→透明，5→未知
    if let Some(level) = query.get("level") && let Some(protocol_type) = query.get("protocol_type") {
        search = format!("ip_cache::{}::{}", protocol_type, level);
    }

    let mut conn = state.redis.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let keys: RedisResult<Vec<String>> =
        AsyncCommands::keys(&mut conn, search).await;
    let keys = match keys {
        Ok(k) => k,
        Err(_) => return Resp::error(404, "ip pool is null"),
    };

    for key in keys {
        let result: RedisResult<HashMap<String, String>> =
            AsyncCommands::hgetall(&mut conn, &key).await;
        match result {
            Ok(map) => {
                for value in map.values() {
                    if let Ok(ip) = serde_json::from_str::<IpDetail>(&value) && check_ip(&ip).await {
                        return Resp::success(ip);
                    }
                }
            }
            Err(_) => {}
        }
    }

    Resp::error(404, "ip pool is null")
}

async fn get_count(state: web::Data<AppState>,) -> impl Responder {
    let mut count = 0;
    let mut conn = state.redis.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let keys: RedisResult<Vec<String>> =
        AsyncCommands::keys(&mut conn, "ip_cache::*").await;
    let keys = match keys {
        Ok(k) => k,
        Err(_) => return Resp::error(404, "ip pool is null"),
    };

    for key in keys {
        count += AsyncCommands::hlen(&mut conn, key).await.unwrap_or(0);
    }

    Resp::success(count)
}

pub(crate) async fn ip_in_redis(redis: Arc<Mutex<ConnectionManager>>, ip_detail: IpDetail) {
    let data = serde_json::to_string(&ip_detail).unwrap_or_else(|_| "".to_string());

    let (key, h_key,mut conn) = get_conn_and_key_data(redis, ip_detail);

    let _: RedisResult<String> = AsyncCommands::hset(&mut conn, &key, &h_key, &data).await;
}

pub async fn remove_ip(redis: Arc<Mutex<ConnectionManager>>, ip_detail: IpDetail) {
    let (key, h_key, mut conn) = get_conn_and_key_data(redis, ip_detail);

    let _: RedisResult<String> = AsyncCommands::hdel(&mut conn, &key, &h_key).await;
}

fn get_conn_and_key_data(redis: Arc<Mutex<ConnectionManager>>, ip_detail: IpDetail) -> (String, String,ConnectionManager) {
    let key = format!("ip_cache::{}::{}", ip_detail.protocol_type, ip_detail.level);

    let h_key = format!("{}:{}", ip_detail.ip, ip_detail.port);

    let conn = redis.lock().unwrap_or_else(|e| e.into_inner()).clone();

    (key, h_key, conn)
}

pub async fn check_ip(ip_detail: &IpDetail) -> bool {
    if ip_detail.ip.is_empty() || ip_detail.port.is_empty() {
        return false;
    }

    if !ip_detail.is_live {
        return false;
    }

    if ip_detail.live_time > 0 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        if now > ip_detail.crawling_time + ip_detail.live_time {
            return false;
        }
    }

    let proxy_url = match ip_detail.protocol_type.as_str() {
        "socks4" => format!("socks4://{}:{}", ip_detail.ip, ip_detail.port),
        "socks5" | "socks" => format!("socks5://{}:{}", ip_detail.ip, ip_detail.port),
        "https" => format!("https://{}:{}", ip_detail.ip, ip_detail.port),
        _ => format!("http://{}:{}", ip_detail.ip, ip_detail.port),
    };

    let proxy = match reqwest::Proxy::all(&proxy_url) {
        Ok(p) => p,
        Err(_) => return false,
    };

    let client = match reqwest::Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    match client.get("https://www.baidu.com").send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

pub(crate) async fn get_all_ips(redis: Arc<Mutex<ConnectionManager>>) -> Vec<IpDetail> {
    let mut ips = Vec::new();
    let mut conn = redis.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let keys: RedisResult<Vec<String>> =
        AsyncCommands::keys(&mut conn, "ip_cache::*").await;
    let keys = match keys {
        Ok(k) => k,
        Err(_) => return ips,
    };

    for key in keys {
        let members: RedisResult<Vec<String>> =
            AsyncCommands::hgetall(&mut conn, &key).await;
        let members = match members {
            Ok(m) => m,
            Err(_) => continue,
        };
        for member in members {
            if let Ok(detail) = serde_json::from_str::<IpDetail>(&member) {
                ips.push(detail);
            }
        }
    }
    ips
}
