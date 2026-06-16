use actix_web::http::header::ContentType;
use actix_web::{HttpRequest, HttpResponse, Responder};
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

pub mod db;
pub mod model;
pub mod scrapy;
pub mod service;

#[derive(Clone)]
pub struct AppState {
    pub redis: Arc<Mutex<ConnectionManager>>,
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
