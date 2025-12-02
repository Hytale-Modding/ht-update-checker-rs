use crate::models::{
    GameVersion, JavaVersion, LauncherVersion, NewGameVersion, NewJavaVersion, NewLauncherVersion,
    UpdateChannel,
};
use crate::schema::java_versions::dsl::java_versions;
use crate::{arcanite, schema};
use anyhow::Context;
use diesel::associations::HasTable;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{
    Connection, Insertable, OptionalExtension, QueryDsl, RunQueryDsl, SelectableHelper,
    SqliteConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::error::Error;

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

pub fn upsert_launcher_version(
    connection: &mut SqliteConnection,
    launcher_version: &str,
) -> anyhow::Result<LauncherVersion> {
    use crate::schema::launcher_versions::dsl::*;

    let result = NewLauncherVersion {
        version: launcher_version,
    }
    .insert_into(launcher_versions)
    .on_conflict_do_nothing()
    .returning(LauncherVersion::as_returning())
    .get_result(connection)
    .with_context(|| format!("upsert_launcher_version @ {launcher_version}"))?;

    Ok(result)
}

pub fn upsert_java_version(
    connection: &mut SqliteConnection,
    java_version: &str,
) -> anyhow::Result<JavaVersion> {
    use crate::schema::java_versions::dsl::*;

    let result = NewJavaVersion {
        version: java_version,
    }
    .insert_into(java_versions)
    .on_conflict_do_nothing()
    .returning(JavaVersion::as_returning())
    .get_result(connection)
    .with_context(|| format!("upsert_java_version @ {java_version}"))?;

    Ok(result)
}

pub fn upsert_game_version(
    connection: &mut SqliteConnection,
    build: u64,
    version_name: &str,
) -> anyhow::Result<GameVersion> {
    use crate::schema::game_versions::dsl::*;

    let result = NewGameVersion {
        build_number: build as i32,
        version: version_name,
    }
    .insert_into(game_versions)
    .on_conflict_do_nothing()
    .returning(GameVersion::as_returning())
    .get_result(connection)
    .with_context(|| format!("upsert_game_version @ {build}"))?;

    Ok(result)
}

pub fn update_channel(
    connection: &mut SqliteConnection,
    ch: &arcanite::UpdateChannel,
    launcher_version_id: &str,
    java_version_id: &str,
    game_version_id: u64,
) -> anyhow::Result<()> {
    // let prev = update_channels.find(ch.to_string());

    Ok(())
}
