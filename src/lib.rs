use crate::arcanite::ArcaniteClient;
use crate::auth::AuthInfo;
use anyhow::anyhow;
use diesel::SqliteConnection;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use reqwest::Client;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use reqwest::redirect::Policy;
use std::path::PathBuf;
use std::str::FromStr;
use tokio::fs;

pub mod arcanite;
pub mod auth;
pub mod db;
pub mod discord;
pub mod models;
pub mod util;
#[rustfmt::skip]
#[allow(clippy::all, warnings)]
pub mod schema;

pub const AUTH_FILE_PATH: &str = "data/accounts.json";

pub fn client() -> anyhow::Result<Client> {
    let mut headers = HeaderMap::with_capacity(2);
    headers.append(ACCEPT, HeaderValue::from_static("application/json"));
    headers.append(USER_AGENT, HeaderValue::from_maybe_shared(user_agent())?);

    Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| anyhow!(e))
}

pub fn user_agent() -> String {
    format!(
        "{product}/{version}",
        product = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    )
}

pub fn auth_client() -> anyhow::Result<Client> {
    Client::builder()
        .user_agent(user_agent())
        .redirect(Policy::none())
        .build()
        .map_err(|e| anyhow!(e))
}

pub struct AppState {
    pub db: Pool<ConnectionManager<SqliteConnection>>,
    pub client: Client,
    pub arcanite: ArcaniteClient,
    pub discord_webhook_url: Option<String>,
}

pub async fn save_auth_info(auth_info: &AuthInfo) -> anyhow::Result<()> {
    let out_path = PathBuf::from_str(AUTH_FILE_PATH)?;

    fs::create_dir_all(out_path.parent().expect("Parent must exist")).await?;
    fs::write(&out_path, serde_json::to_string(&auth_info)?).await?;

    Ok(())
}
