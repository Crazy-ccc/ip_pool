use fred::prelude::*;
use std::env;

pub async fn connect_redis() -> Result<Pool, Error> {
    let url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let config = Config::from_url(&url)?;
    let pool = Builder::from_config(config).build_pool(4)?;
    pool.init().await?;
    Ok(pool)
}