use crate::auth::AuthInfo;
use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT};
use reqwest::redirect::Policy;
use reqwest::Client;

pub mod arcanite;
pub mod auth;
pub mod discord;
pub mod util;
pub mod db;

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
    pub auth: AuthInfo,
    pub client: Client,
    pub discord_webhook_url: Option<String>,
}
