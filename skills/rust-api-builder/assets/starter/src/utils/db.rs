use actix_web::web;
use diesel::pg::PgConnection;

use crate::config::config_service::DbPool;

/// Error type of every repository method. `Send + Sync` so it can cross `web::block`.
pub type DbError = Box<dyn std::error::Error + Send + Sync>;

/// Runs blocking Diesel work on actix's blocking thread pool with a pooled connection, so database
/// calls never stall the async workers. Everything the closure uses must be moved into it.
pub async fn run_blocking<F, R>(pool: &DbPool, f: F) -> Result<R, DbError>
where
    F: FnOnce(&mut PgConnection) -> Result<R, DbError> + Send + 'static,
    R: Send + 'static,
{
    let pool = pool.clone();

    web::block(move || {
        let mut conn = pool.get()?;
        f(&mut conn)
    })
    .await?
}
