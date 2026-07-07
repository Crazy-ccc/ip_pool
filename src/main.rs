use std::env;
use actix_web::{App, HttpResponse, HttpServer, guard, web};
use ip_pool::AppState;
use ip_pool::db::redis::connect_redis;
use ip_pool::service::{ip_cache, task};
use ip_pool::service::pool::Pool;
use log::error;
use std::sync::{Arc, Mutex};

const DEFAULT_POOL_SIZE: usize = 4;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    unsafe {
        env::set_var("RUST_LOG", "actix_web=info,ip_pool=info");
    }
    env_logger::init();

    let redis = match connect_redis().await {
        Ok(redis) => redis,
        Err(e) => {
            error!("connect redis error, {}", e);
            std::process::exit(1);
        }
    };

    let redis = Arc::new(Mutex::new(redis));
    let size: usize = env::var("POOL_SIZE").map(|s| { s.parse().unwrap_or(DEFAULT_POOL_SIZE) }).unwrap_or(DEFAULT_POOL_SIZE);
    let pool = Pool::new(size);

    task::start(redis.clone(), pool);

    let state = web::Data::new(AppState { redis });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .default_service(
                web::route()
                    .guard(guard::Not(guard::Get()))
                    .to(|| HttpResponse::MethodNotAllowed()),
            )
            .service(ip_cache::service())
    })
    .workers(2)
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
