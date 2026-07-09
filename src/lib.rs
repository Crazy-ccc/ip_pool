use actix_web::http::header::ContentType;
use actix_web::{HttpRequest, HttpResponse, Responder};
use fred::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub mod db;
pub mod model;
pub mod scrapy;
pub mod service;

#[derive(Clone)]
pub struct LiveKeyCache {
    pub data: Arc<RwLock<HashMap<String, Instant>>>,
    pub ttl: Duration,
}

impl LiveKeyCache {
    pub fn new(ttl_secs: u64) -> Self {
        LiveKeyCache {
            data: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    pub async fn get_or_refresh(
        &self,
        redis: &Pool,
        pattern: &str,
    ) -> Result<Vec<String>, Error> {
        // Check if cache is valid
        {
            let data = self.data.read().unwrap();
            if !data.is_empty() {
                if let Some(first_time) = data.values().next() {
                    if first_time.elapsed() < self.ttl {
                        // Cache is valid, return keys matching pattern
                        let keys: Vec<String> = data
                            .keys()
                            .filter(|k| glob_match(pattern, k))
                            .cloned()
                            .collect();
                        return Ok(keys);
                    }
                }
            }
        }

        // Cache expired or empty, refresh
        let keys = crate::service::ip_cache::scan_keys_public(redis, pattern).await?;

        {
            let mut data = self.data.write().unwrap();
            let now = Instant::now();
            data.clear();
            for key in &keys {
                data.insert(key.clone(), now);
            }
        }

        Ok(keys)
    }
}

#[derive(Clone)]
pub struct AppState {
    pub redis: Pool,
    pub live_key_cache: LiveKeyCache,
}

// response body
#[derive(Deserialize, Serialize)]
struct Resp<T: Serialize> {
    code: i32,
    msg: String,
    data: Option<T>,
}

impl<T: Serialize> Responder for Resp<T> {
    type Body = actix_web::body::BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        let body = serde_json::to_string(&self).unwrap();

        HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(body)
    }
}

impl<T: Serialize> Resp<T> {
    pub fn success(data: T) -> Self {
        Resp {
            code: 0,
            msg: String::new(),
            data: Some(data),
        }
    }

    pub fn error(code: i32, msg: &str) -> Self {
        Resp {
            code,
            msg: String::from(msg),
            data: None,
        }
    }
}


pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn glob_match(pattern: &str, s: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == s;
    }
    if !s.starts_with(parts[0]) {
        return false;
    }
    let mut pos = parts[0].len();
    for (i, part) in parts.iter().enumerate().skip(1) {
        if i == parts.len() - 1 {
            return s[pos..].ends_with(part);
        }
        match s[pos..].find(part) {
            Some(idx) => pos += idx + part.len(),
            None => return false,
        }
    }
    true
}

