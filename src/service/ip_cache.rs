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
    // protocol_type(可选)：代理协议，http，socks4，socks5，https
    let protocol_type = query
        .get("protocol_type")
        .map(|t| t.as_str())
        .unwrap_or("http");
    // 1→高匿，2→普匿，3→匿名，4→透明，5→未知
    let level = query.get("level").map(|t| t.as_str()).unwrap_or("1");
    let num = query.get("num").map(|t| t.parse().unwrap_or(1)).unwrap_or(1);

    let mut results: Vec<IpDetail> = vec![];
    for _i in 0..(10 * num) {
        match get_ip_for_redis(state.redis.clone(), protocol_type, level).await {
            Some(ip) => {
                if check_ip(&ip).await {
                    results.push(ip);
                } else {
                    let _ = remove_ip(state.redis.clone(), ip).await;
                }
            }
            _ => {}
        }
    }

    Resp::success(results)
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
        count += AsyncCommands::scard(&mut conn, key).await.unwrap_or(0);
    }

    Resp::success(count)
}

async fn get_ip_for_redis(
    redis: Arc<Mutex<ConnectionManager>>,
    protocol_type: &str,
    level: &str,
) -> Option<IpDetail> {
    let mut conn = redis.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let key = format!("ip_cache::{}::{}", protocol_type, level);
    let result: RedisResult<String> = AsyncCommands::srandmember(&mut conn, key).await;
    result.ok().and_then(|r| serde_json::from_str(&r).ok())
}

pub(crate) async fn ip_in_redis(redis: Arc<Mutex<ConnectionManager>>, ip_detail: IpDetail) {
    let (data, key, mut conn) = get_conn_and_key_data(redis, ip_detail);

    let _: RedisResult<String> = AsyncCommands::sadd(&mut conn, &key, &data).await;
}

pub async fn remove_ip(redis: Arc<Mutex<ConnectionManager>>, ip_detail: IpDetail) {
    let (data, key, mut conn) = get_conn_and_key_data(redis, ip_detail);

    let _: RedisResult<String> = AsyncCommands::srem(&mut conn, &key, &data).await;
}

pub fn get_conn_and_key_data(redis: Arc<Mutex<ConnectionManager>>, ip_detail: IpDetail) -> (String, String, ConnectionManager) {
    let data = serde_json::to_string(&ip_detail).unwrap_or_else(|_| "".to_string());

    let key = format!("ip_cache::{}::{}", ip_detail.protocol_type, ip_detail.level);

    let conn = redis.lock().unwrap_or_else(|e| e.into_inner()).clone();

    (data, key, conn)
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
        .timeout(Duration::from_secs(5))
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
            AsyncCommands::smembers(&mut conn, &key).await;
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
