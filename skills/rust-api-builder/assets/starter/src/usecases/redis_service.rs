use redis::aio::ConnectionManager;
use redis::{AsyncCommands, RedisError};

pub struct RedisService<'a> {
    redis: &'a ConnectionManager,
}

impl<'a> RedisService<'a> {
    pub fn new(redis: &'a ConnectionManager) -> Self {
        RedisService { redis }
    }

    pub async fn set(&self, key: &str, value: &str, ttl_seconds: Option<u64>) -> Result<(), RedisError> {
        let mut conn = self.redis.clone();
        match ttl_seconds {
            Some(ttl) => conn.set_ex::<_, _, ()>(key, value, ttl).await,
            None => conn.set::<_, _, ()>(key, value).await,
        }
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>, RedisError> {
        let mut conn = self.redis.clone();
        conn.get::<_, Option<String>>(key).await
    }

    pub async fn delete(&self, key: &str) -> Result<(), RedisError> {
        let mut conn = self.redis.clone();
        conn.del::<_, ()>(key).await
    }

    /// Deletes every key matching `pattern` (e.g. `product:list:ACME:*`) using SCAN, not KEYS.
    pub async fn delete_pattern(&self, pattern: &str) -> Result<(), RedisError> {
        let mut scan_conn = self.redis.clone();
        let mut keys: Vec<String> = Vec::new();
        {
            let mut iter = scan_conn.scan_match::<_, String>(pattern).await?;
            while let Some(key) = iter.next_item().await {
                keys.push(key?);
            }
        }

        if keys.is_empty() {
            return Ok(());
        }

        let mut conn = self.redis.clone();
        conn.del::<_, ()>(keys).await
    }

    pub async fn ping(&self) -> Result<(), RedisError> {
        let mut conn = self.redis.clone();
        redis::cmd("PING").query_async::<String>(&mut conn).await.map(|_| ())
    }
}
