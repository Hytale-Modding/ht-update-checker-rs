use std::error::Error;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub fn apply_migrations(
    pool: &Pool<ConnectionManager<SqliteConnection>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

    let mut connection = pool.get()?;

    if connection.has_pending_migration(MIGRATIONS)? {
        log::info!("Running database migrations...");
        connection.run_pending_migrations(MIGRATIONS)?;
        log::info!("done!");
    }

    Ok(())
}