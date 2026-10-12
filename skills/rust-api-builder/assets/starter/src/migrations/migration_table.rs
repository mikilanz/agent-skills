use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

use crate::config::config_service::DbPool;
use crate::utils::db::DbError;

// compiled into the binary, so the image does not need the migrations folder at runtime
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub fn run_migrations(pool: &DbPool) -> Result<(), DbError> {
    let mut connection = pool.get()?;

    for version in connection.run_pending_migrations(MIGRATIONS)? {
        log::info!("Applied migration {}", version);
    }

    Ok(())
}
