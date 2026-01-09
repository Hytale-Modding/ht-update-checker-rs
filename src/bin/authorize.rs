use anyhow::{Context, bail};
use base64::Engine;
use base64::prelude::BASE64_URL_SAFE;
use chrono::Utc;
use env_logger::Env;
use ht_update_checker::arcanite;
use ht_update_checker::auth::{AccessTokenInfo, AuthInfo};
use oauth2::url::Url;
use oauth2::{
    AuthorizationCode, CsrfToken, DeviceAuthorizationResponse, EmptyExtraDeviceAuthorizationFields,
    PkceCodeChallenge, Scope, TokenResponse,
};
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

#[dotenvy::load(required = false)]
#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    let mut client = ht_update_checker::auth_client()?;

    let arg0 = std::env::args().nth(1);
    let auth_mode = if arg0.is_some_and(|value| value == "--device-auth") {
        AuthMode::DeviceAuth
    } else {
        AuthMode::default()
    };
    let auth_info = match auth_mode {
        AuthMode::OAuth => browser_auth(&mut client).await?,
        AuthMode::DeviceAuth => device_auth(&mut client).await?,
    };

    ht_update_checker::save_auth_info(&auth_info)
        .await
        .context("Unable to save new authentication info")?;

    log::info!("Success!");

    Ok(())
}

#[derive(Default)]
enum AuthMode {
    #[default]
    OAuth,
    DeviceAuth,
}

async fn device_auth(client: &mut Client) -> anyhow::Result<AuthInfo> {
    let auth_client = arcanite::oauth_client();

    let response: DeviceAuthorizationResponse<EmptyExtraDeviceAuthorizationFields> = auth_client
        .exchange_device_code()
        .add_scopes(vec![
            Scope::new("openid".to_string()),
            Scope::new("offline".to_string()),
        ])
        .request_async(client)
        .await
        .context("Failed to request device verification code")?;

    log::info!("=================================");
    log::info!(
        "Visit {authorization_url}",
        authorization_url = response
            .verification_uri_complete()
            .map(|url| url.secret())
            .unwrap_or_else(|| response.verification_uri())
    );
    log::info!(
        "Enter Code: {authorization_code}",
        authorization_code = response.user_code().secret()
    );
    log::info!("=================================");
    log::info!(
        "Waiting for login... (expires in {exp:?})",
        exp = response.expires_in()
    );

    let auth_response = auth_client
        .exchange_device_access_token(&response)
        .request_async(client, tokio::time::sleep, Some(response.expires_in()))
        .await
        .context("Failed to retrieve device auth token")?;

    log::info!("Received authorization token.");

    let Some(refresh_token) = auth_response.refresh_token() else {
        bail!("No refresh token received!");
    };

    let scopes: Vec<String> = auth_response
        .scopes()
        .unwrap_or(&vec![])
        .iter()
        .map(|s| s.as_str().to_string())
        .collect();

    let auth_token = auth_response.access_token().secret().clone();

    let Some(token_expiry) = auth_response.expires_in().map(|dur| Utc::now() + dur) else {
        bail!("No token lifetime received!");
    };

    let auth_info = AuthInfo {
        auth_token: Some(AccessTokenInfo {
            token: auth_token,
            expires_at: token_expiry,
            token_type: auth_response.token_type().into(),
        }),
        refresh_token: refresh_token.secret().clone(),
        scopes,
    };

    Ok(auth_info)
}

async fn browser_auth(client: &mut Client) -> anyhow::Result<AuthInfo> {
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct StatePayload {
        state: String,
        port: u16,
    }

    impl StatePayload {
        pub fn csrf_token(&self) -> CsrfToken {
            CsrfToken::new(self.state.clone())
        }
    }

    let mut rng = StdRng::from_os_rng();
    let local_port = rng.random_range(30000..=60000) as u16;

    let auth_client = arcanite::oauth_client();

    // pass the target port along in the CSRF token, but do remember we need to compare against
    // the unwrapped value!
    let csrf_state_payload = StatePayload {
        state: CsrfToken::new_random().into_secret(),
        port: local_port,
    };
    let state_str = BASE64_URL_SAFE.encode(serde_json::to_string(&csrf_state_payload)?);

    let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();
    let (authorize_url, csrf_state) = auth_client
        .authorize_url(|| CsrfToken::new(state_str))
        .set_pkce_challenge(pkce_code_challenge)
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("offline".to_string()))
        .url();

    log::info!("Opening web browser...");
    webbrowser::open(authorize_url.as_str()).context("Unable to open default web browser")?;

    let (code, state) = {
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, local_port))
            .await
            .with_context(|| format!("Unable to start TCP listener on 127.0.0.1:{local_port}"))?;

        let (mut stream, _addr) = listener
            .accept()
            .await
            .context("Listener terminated without accepting a connection!")?;

        let mut reader = BufReader::new(&mut stream);

        let mut req_line = String::new();
        reader.read_line(&mut req_line).await?;

        let red_url = req_line
            .split_ascii_whitespace()
            .nth(1)
            .expect("Invalid request line!");
        let url = Url::parse(format!("http://localhost{red_url}").as_str())?;

        let code = url
            .query_pairs()
            .find(|(k, _v)| k == "code")
            .map(|(_k, v)| AuthorizationCode::new(v.into_owned()))
            .expect("No code verifier provided!");

        let state = url
            .query_pairs()
            .find(|(k, _v)| k == "state")
            .map(|(_k, v)| CsrfToken::new(v.into_owned()))
            .expect("No CSRF token provided!");

        let message = "You may close this window now.";
        let http_response = format!(
            "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{message}",
            message.len()
        );
        stream.write_all(http_response.as_bytes()).await?;

        (code, state)
    };

    if state != csrf_state_payload.csrf_token() {
        log::info!("received state: {}", state.secret());
        log::info!("expected state: {}", csrf_state.secret());

        bail!("CSRF secret mismatch, please try again!");
    }

    log::info!("Received authorization code, exchanging for token...");

    let auth_response = auth_client
        .exchange_code(code)
        .set_pkce_verifier(pkce_code_verifier)
        .request_async(client)
        .await
        .context("Unable to exchange authorization code for access token")?;

    let Some(refresh_token) = auth_response.refresh_token() else {
        bail!("No refresh token received!");
    };

    let scopes: Vec<String> = auth_response
        .scopes()
        .unwrap_or(&vec![])
        .iter()
        .map(|s| s.as_str().to_string())
        .collect();

    let auth_token = auth_response.access_token().secret().clone();

    let Some(token_expiry) = auth_response.expires_in().map(|dur| Utc::now() + dur) else {
        bail!("No token lifetime received!");
    };

    let auth_info = AuthInfo {
        auth_token: Some(AccessTokenInfo {
            token: auth_token,
            expires_at: token_expiry,
            token_type: auth_response.token_type().into(),
        }),
        refresh_token: refresh_token.secret().clone(),
        scopes,
    };

    Ok(auth_info)
}
