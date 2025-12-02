use anyhow::{Context, anyhow, bail};
use diesel::{SqliteConnection};
use diesel::r2d2::{ConnectionManager, Pool};
use env_logger::Env;
use ht_update_checker::arcanite::{TargetArchitecture, TargetOs, UpdateChannel};
use ht_update_checker::auth::AuthInfo;
use ht_update_checker::{AppState, arcanite, client, db, discord};
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio_cron_scheduler::job::JobId;
use tokio_cron_scheduler::{Job, JobScheduler};

const DEFAULT_SCHEDULE: &str = "every 30 seconds";

#[dotenvy::load(required = false)]
#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let accounts_store_path = PathBuf::from_str(ht_update_checker::AUTH_FILE_PATH)?;
    let Ok(stream) = File::open(&accounts_store_path) else {
        log::error!(
            "No auth credentials found at {path}",
            path = accounts_store_path.to_string_lossy()
        );
        return Ok(());
    };

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<SqliteConnection>::new(database_url.clone());
    let pool: Pool<ConnectionManager<SqliteConnection>> = Pool::builder()
        .build(manager)
        .with_context(|| format!("Failed to connect to database at {database_url}"))?;
    db::apply_migrations(&pool).map_err(|e| anyhow!(e))?;

    let reader = BufReader::new(stream);
    let account: AuthInfo =
        serde_json::from_reader(reader).context("Unable to deserialize auth info")?;

    let discord_webhook_url = env::var("DISCORD_WEBHOOK_URL").ok();
    if discord_webhook_url.is_none() {
        log::warn!("DISCORD_WEBHOOK_URL is not set!");
    }

    let state = Arc::new(AppState {
        auth: account,
        db: pool,
        client: client()?,
        discord_webhook_url,
    });

    let scheduler = JobScheduler::new().await?;
    scheduler.shutdown_on_ctrl_c();

    let update_schedule = env::var("UPDATE_SCHEDULE")
        .ok()
        .unwrap_or_else(|| DEFAULT_SCHEDULE.to_string());

    log::info!("Job scheduled at interval: {update_schedule}");

    scheduler
        .add(Job::new_one_shot_async(Duration::from_secs(0), {
            let state = Arc::clone(&state);
            move |job_id, _scheduler| {
                let state = Arc::clone(&state);
                Box::pin(async move { try_update(job_id, &state).await })
            }
        })?)
        .await?;

    scheduler
        .add(Job::new_cron_job_async(&update_schedule, {
            let state = Arc::clone(&state);
            move |job_id, _scheduler| {
                let state = Arc::clone(&state);
                Box::pin(async move { try_update(job_id, &state).await })
            }
        })?)
        .await?;

    scheduler.start().await?;

    tokio::signal::ctrl_c().await?;
    Ok(())
}

async fn try_update(job_id: JobId, state: &AppState) {
    log::info!("Start executing job {job_id}");

    if let Err(e) = do_update(&job_id, state).await {
        log::error!("Error while running update: {e:#}");
    }
}

async fn do_update(_job_id: &JobId, state: &AppState) -> anyhow::Result<()> {
    let mut db = state.db.get()?;
    let channel = UpdateChannel::default();

    let launcher_meta = arcanite::get_launcher_meta(&state.client, &channel).await?;
    db::upsert_launcher_version(&mut db, &launcher_meta.version)?;

    let java_meta = arcanite::get_java_version_meta(&state.client, &launcher_meta).await?;
    db::upsert_java_version(&mut db, &java_meta.version)?;

    let account_meta = arcanite::get_account_meta(
        &state.client,
        &state.auth,
        &launcher_meta,
        TargetOs::Windows,
        TargetArchitecture::Amd64,
    )
    .await?;

    let Some(patch_info) = account_meta.patchlines.get(&channel) else {
        bail!("Unable to find patchline {channel}");
    };
    db::upsert_game_version(&mut db, patch_info.current_build, &patch_info.build_version)?;

    // db::update_channel(&mut db, &channel);

    log::info!(
        "Launcher version: {launcher_version}; Java version: {java_version}, game version: {game_version} build: {build_number}",
        launcher_version = launcher_meta.version,
        java_version = java_meta.version,
        game_version = patch_info.build_version,
        build_number = patch_info.current_build
    );

    if state.discord_webhook_url.is_some() {
        discord::send_webhook(state, &launcher_meta, &java_meta, patch_info).await?;
    }

    Ok(())
}
