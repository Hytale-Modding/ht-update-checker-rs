use crate::arcanite;
use crate::models::{
    GameVersion, JavaVersion, LauncherVersion, NewGameVersion, NewJavaVersion, NewLauncherVersion,
    NewUpdateChannel, UpdateChannel,
};
use anyhow::{Context, bail};
use diesel::dsl::now;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{
    ExpressionMethods, Insertable, OptionalEmptyChangesetExtension, OptionalExtension, QueryDsl,
    RunQueryDsl, SelectableHelper, SqliteConnection,
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use std::error::Error;
use std::num::NonZeroU64;

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
    new_launcher_version: &str,
) -> anyhow::Result<LauncherVersion> {
    use crate::schema::launcher_versions::dsl::*;

    let result = NewLauncherVersion {
        version: new_launcher_version,
    }
    .insert_into(launcher_versions)
    .on_conflict_do_nothing()
    .returning(LauncherVersion::as_returning())
    .get_result(connection)
    .optional()
    .with_context(|| format!("upsert_launcher_version @ {new_launcher_version}"))?;

    match result {
        Some(result) => Ok(result),
        None => {
            let v = launcher_versions
                .find(new_launcher_version)
                .select(LauncherVersion::as_select())
                .first(connection)
                .with_context(|| {
                    format!("upsert_launcher_version::find @ {new_launcher_version}")
                })?;

            Ok(v)
        }
    }
}

pub fn upsert_java_version(
    connection: &mut SqliteConnection,
    new_java_version: &str,
) -> anyhow::Result<JavaVersion> {
    use crate::schema::java_versions::dsl::*;

    let result = NewJavaVersion {
        version: new_java_version,
    }
    .insert_into(java_versions)
    .on_conflict_do_nothing()
    .returning(JavaVersion::as_returning())
    .get_result(connection)
    .optional()
    .with_context(|| format!("upsert_java_version @ {new_java_version}"))?;

    match result {
        Some(result) => Ok(result),
        None => {
            let v = java_versions
                .find(new_java_version)
                .select(JavaVersion::as_select())
                .first(connection)
                .with_context(|| format!("upsert_java_version::find @ {new_java_version}"))?;

            Ok(v)
        }
    }
}

pub fn upsert_game_version(
    connection: &mut SqliteConnection,
    build: NonZeroU64,
    version_name: &str,
) -> anyhow::Result<GameVersion> {
    use crate::schema::game_versions::dsl::*;

    let result = NewGameVersion {
        build_number: build.get() as i32,
        version: version_name,
    }
    .insert_into(game_versions)
    .on_conflict_do_nothing()
    .returning(GameVersion::as_returning())
    .get_result(connection)
    .optional()
    .with_context(|| format!("upsert_game_version @ {build}"))?;

    match result {
        Some(result) => Ok(result),
        None => {
            let v = game_versions
                .find(build.get() as i32)
                .select(GameVersion::as_select())
                .first(connection)
                .with_context(|| format!("upsert_game_version::find @ {build}"))?;

            Ok(v)
        }
    }
}

pub fn update_channel(
    connection: &mut SqliteConnection,
    ch: &arcanite::UpdateChannel,
    current_launcher_version: &LauncherVersion,
    current_java_version: &JavaVersion,
    current_game_version: &GameVersion,
) -> anyhow::Result<Option<(Option<UpdateChannel>, UpdateChannel)>> {
    use crate::schema::update_channels::dsl::*;

    let channel_name = ch.to_string();
    let prev = update_channels
        .find(&channel_name)
        .select(UpdateChannel::as_select())
        .first(connection)
        .optional()
        .context("update_channel::find")?;

    if prev.as_ref().is_some_and(|prev| {
        prev.launcher_version_id == current_launcher_version.version
            && prev.java_version_id == current_java_version.version
            && prev.game_version_id == current_game_version.build_number
    }) {
        return Ok(None);
    }

    let data = NewUpdateChannel {
        channel: &ch.to_string(),

        launcher_version_id: &current_launcher_version.version,
        java_version_id: &current_java_version.version,
        game_version_id: current_game_version.build_number,
    };

    let affected = diesel::insert_into(update_channels)
        .values(&data)
        .on_conflict(channel)
        .do_update()
        .set(&data)
        .execute(connection)
        .optional_empty_changeset()
        .context("update_channel::upsert")?
        .unwrap_or(0);

    if affected > 1 {
        log::error!(
            "update_channel::upsert DATABASE ERROR: got {affected} affected rows, expected at most 1"
        );
        bail!("")
    } else if affected > 0 {
        let result = diesel::update(update_channels)
            .filter(channel.eq(data.channel))
            .set(updated_at.eq(now))
            .returning(UpdateChannel::as_returning())
            .get_result(connection)
            .context("update_channel::update_timestamp")?;

        return Ok(Some((prev, result)));
    }

    Ok(None)
}
