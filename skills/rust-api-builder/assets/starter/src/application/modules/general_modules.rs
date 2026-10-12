use redis::aio::ConnectionManager as RedisConnectionManager;
use tokio_cron_scheduler::JobScheduler;

use crate::config::config_service::{ConfigService, DbPool};

/// Application state shared by every request (`web::Data<GeneralModules>`). Everything in it is
/// cheap to clone: the pool, redis manager, http client and scheduler are handles to shared state.
#[derive(Clone)]
pub struct GeneralModules {
    pub app_name: String,
    pub pool: DbPool,
    pub config_service: ConfigService,
    pub redis: RedisConnectionManager,
    pub http_client: reqwest::Client,
    pub scheduler: JobScheduler,
    // repositories: one field per table, each built with `pool.clone()` in lib.rs
}
