use redis::aio::ConnectionManager;
use std::env;

pub async fn connect_redis() -> Result<ConnectionManager, redis::RedisError> {
    let url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let client = redis::Client::open(url)?;
    ConnectionManager::new(client).await
}
