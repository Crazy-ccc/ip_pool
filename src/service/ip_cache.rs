use crate::model::ip_detail::IpDetail;
use crate::{AppState, Resp};
use actix_web::{Responder, Scope, web};
use fred::prelude::*;
use fred::types::CustomCommand;
use std::collections::HashMap;
use std::time::Duration;
use rand::prelude::IteratorRandom;

const VERIFY_TARGETS: &[&str] = &[
    "https://www.baidu.com",
    "https://httpbin.org/ip",
    "http://www.google.com",
];

fn is_valid_ip(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().ok().is_some())
}

fn is_valid_port(s: &str) -> bool {
    s.parse::<u16>().map_or(false, |p| p > 0)
}

fn live_set_key(protocol_type: &str, level: &str) -> String {
    format!("ip_live::{}::{}", protocol_type, level)
}

fn cache_key(protocol_type: &str, level: &str) -> String {
    format!("ip_cache::{}::{}", protocol_type, level)
}

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
    let protocol_type = query.get("protocol_type");
    let level = query.get("level");

    // Direct lookup when both protocol_type and level are specified
    if let (Some(pt), Some(lv)) = (protocol_type, level) {
        let set_key = live_set_key(pt, lv);
        return match try_get_ip_from_set(&state.redis, &set_key, pt, lv).await {
            Some(ip) => Resp::success(ip),
            None => Resp::error(404, "ip pool is null"),
        };
    }

    // Wildcard pattern: get matching live set keys
    let live_set_pattern = match protocol_type {
        Some(pt) => format!("ip_live::{}::*", pt),
        None => "ip_live::*".to_string(),
    };

    let keys = match get_keys(&state.redis, &live_set_pattern).await {
        Ok(k) if !k.is_empty() => k,
        _ => return Resp::error(404, "ip pool is null"),
    };

    let mut rng = rand::rng();
    let keys: Vec<String> = keys.iter().filter_map(|k| k.as_str().map(|s| s.to_string())).collect();

    for _ in 0..keys.len() {
        if let Some(key) = keys.iter().choose(&mut rng) {
            let parts: Vec<&str> = key.split("::").collect();
            if parts.len() >= 3 {
                let pt = parts[1];
                let lv = parts[2];
                if let Some(ip) = try_get_ip_from_set(&state.redis, key, pt, lv).await {
                    return Resp::success(ip);
                }
            }
        }
    }

    Resp::error(404, "ip pool is null")
}

async fn try_get_ip_from_set(redis: &Pool, set_key: &str, protocol_type: &str, level: &str) -> Option<IpDetail> {
    let member: Option<String> = redis.srandmember(set_key, Some(1usize)).await.ok().flatten();
    let member = member?;

    let cache_key = cache_key(protocol_type, level);
    let value: Option<String> = redis.hget(&cache_key, &member).await.ok().flatten();
    let value = value?;

    serde_json::from_str::<IpDetail>(&value).ok()
}

async fn get_count(state: web::Data<AppState>) -> impl Responder {
    let keys = match get_keys(&state.redis, "ip_live::*").await {
        Ok(k) => k,
        Err(_) => return Resp::error(404, "ip pool is null"),
    };

    let mut count = 0i64;
    for key in keys {
        if let Ok(len) = state.redis.scard::<i64, _>(key).await {
            count += len;
        }
    }

    Resp::success(count as usize)
}

pub async fn get_live_count(redis: Pool) -> usize {
    let keys = match get_keys(&redis, "ip_live::*").await {
        Ok(k) => k,
        Err(_) => return 0,
    };
    let mut count = 0i64;
    for key in keys {
        if let Ok(len) = redis.scard::<i64, _>(key).await {
            count += len;
        }
    }
    count as usize
}

pub(crate) async fn ip_in_redis(redis: Pool, ip_detail: IpDetail) {
    let data = serde_json::to_string(&ip_detail).unwrap_or_else(|_| "".to_string());
    let key = cache_key(&ip_detail.protocol_type, &ip_detail.level);
    let h_key = format!("{}:{}", ip_detail.ip, ip_detail.port);

    let _ = redis.hset::<i64, _, _>(&key, (&h_key, &data)).await;

    let live_key = live_set_key(&ip_detail.protocol_type, &ip_detail.level);
    if ip_detail.is_live {
        let _ = redis.sadd::<i64, _, _>(&live_key, &h_key).await;
    } else {
        let _ = redis.srem::<i64, _, _>(&live_key, &h_key).await;
    }
}

pub async fn remove_ip(redis: Pool, ip_detail: IpDetail) {
    let key = cache_key(&ip_detail.protocol_type, &ip_detail.level);
    let h_key = format!("{}:{}", ip_detail.ip, ip_detail.port);

    let _ = redis.hdel::<i64, _, _>(&key, &h_key).await;

    let live_key = live_set_key(&ip_detail.protocol_type, &ip_detail.level);
    let _ = redis.srem::<i64, _, _>(&live_key, &h_key).await;
}

pub async fn check_ip(ip_detail: &IpDetail) -> bool {
    if ip_detail.ip.is_empty()
        || ip_detail.port.is_empty()
        || !is_valid_ip(&ip_detail.ip)
        || !is_valid_port(&ip_detail.port)
    {
        return false;
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
        .timeout(Duration::from_secs(8))
        .connect_timeout(Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    let h1 = tokio::spawn({
        let c = client.clone();
        async move { c.get(VERIFY_TARGETS[0]).send().await.map_or(false, |r| r.status().is_success()) }
    });
    let h2 = tokio::spawn({
        let c = client.clone();
        async move { c.get(VERIFY_TARGETS[1]).send().await.map_or(false, |r| r.status().is_success()) }
    });
    let h3 = tokio::spawn({
        let c = client.clone();
        async move { c.get(VERIFY_TARGETS[2]).send().await.map_or(false, |r| r.status().is_success()) }
    });

    h1.await.unwrap_or(false) || h2.await.unwrap_or(false) || h3.await.unwrap_or(false)
}

pub(crate) async fn get_all_ips(redis: Pool) -> Vec<IpDetail> {
    let mut ips = Vec::new();

    let keys = match scan_keys(&redis, "ip_cache::*").await {
        Ok(k) => k,
        Err(_) => return ips,
    };

    for key in keys {
        let members: Result<HashMap<String, String>, Error> = redis.hgetall(&key).await;
        let members = match members {
            Ok(m) => m,
            Err(_) => continue,
        };
        for (_, value) in members {
            if let Ok(detail) = serde_json::from_str::<IpDetail>(&value) {
                ips.push(detail);
            }
        }
    }
    ips
}

/// Uses the Redis KEYS command to find all matching keys.
async fn get_keys(pool: &Pool, pattern: &str) -> Result<Vec<Key>, Error> {
    let cmd = CustomCommand::new_static("KEYS", fred::types::ClusterHash::default(), false);
    pool.custom(cmd, vec![pattern]).await
}

/// Scan keys using SCAN command (production-safe).
/// Uses scan_page to iterate through all matching keys.
async fn scan_keys(pool: &Pool, pattern: &str) -> Result<Vec<String>, Error> {
    let mut keys = Vec::new();
    let mut cursor = "0".to_string();
    loop {
        let result: Value = pool.scan_page(cursor.clone(), pattern, Some(100u32), None).await?;
        match result {
            Value::Array(ref arr) if arr.len() >= 2 => {
                if let Some(c) = arr[0].as_str() {
                    cursor = c.to_string();
                }
                if let Value::Array(ref key_arr) = arr[1] {
                    for key_val in key_arr {
                        if let Some(s) = key_val.as_str() {
                            keys.push(s.to_string());
                        }
                    }
                }
            }
            _ => break,
        }
        if cursor == "0" {
            break;
        }
    }
    Ok(keys)
}