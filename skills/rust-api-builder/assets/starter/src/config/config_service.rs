use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use redis::aio::ConnectionManager as RedisConnectionManager;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct ConfigService {}

impl ConfigService {
    pub fn get_pool_connection_db(&self) -> DbPool {
        let database_url = self.get_env("DATABASE_URL");
        let max_size = self
            .get_env_or("DATABASE_POOL_SIZE", "5")
            .parse::<u32>()
            .expect("DATABASE_POOL_SIZE must be a number");

        let manager = ConnectionManager::<PgConnection>::new(database_url);

        Pool::builder()
            .max_size(max_size)
            .build(manager)
            .expect("Failed to create pool.")
    }

    pub async fn get_redis_connection(&self) -> RedisConnectionManager {
        let redis_url = self.get_env("REDIS_URL");

        let client = redis::Client::open(redis_url).expect("REDIS_URL must be a valid redis url");

        RedisConnectionManager::new(client)
            .await
            .expect("Failed to connect to redis.")
    }

    pub fn get_env(&self, env_key: &str) -> String {
        let error_msg = format!("{} must be set", env_key);
        dotenvy::var(env_key).expect(&error_msg)
    }

    pub fn get_env_or(&self, env_key: &str, default: &str) -> String {
        dotenvy::var(env_key).unwrap_or_else(|_| default.to_owned())
    }
}
