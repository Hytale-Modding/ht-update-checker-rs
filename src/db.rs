use crate::arcanite;
use crate::models::{
    GameVersion, JavaVersion, LauncherVersion, NewGameVersion, NewJavaVersion, NewLauncherVersion,
    NewUpdateChannel, SetUpdateChannel, UpdateChannel,
};
use anyhow::Context;
use chrono::Utc;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{Insertable, OptionalExtension, RunQueryDsl, SelectableHelper, SqliteConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
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
    current_launcher_version: &LauncherVersion,
    current_java_version: &JavaVersion,
    current_game_version: &GameVersion,
) -> anyhow::Result<Option<UpdateChannel>> {
    use crate::schema::update_channels::dsl::*;

    let data = NewUpdateChannel {
        channel: &ch.to_string(),

        launcher_version_id: &current_launcher_version.version,
        java_version_id: &current_java_version.version,
        game_version_id: current_game_version.build_number,

        ..Default::default()
    };

    let result = diesel::insert_into(update_channels)
        .values(&data)
        .on_conflict(channel)
        .do_update()
        .set(SetUpdateChannel::convert(&data, &Utc::now()))
        .returning(UpdateChannel::as_returning())
        .get_result(connection)
        .optional()
        .context("update_channel")?;

    Ok(result)
}
