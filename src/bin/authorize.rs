use anyhow::{Context, bail};
use chrono::Utc;
use env_logger::Env;
use ht_update_checker::arcanite;
use ht_update_checker::auth::{AccessTokenInfo, AuthInfo};
use oauth2::{
    DeviceAuthorizationResponse, EmptyExtraDeviceAuthorizationFields, Scope, TokenResponse,
};
use reqwest::Client;

#[dotenvy::load(required = false)]
#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    let mut client = ht_update_checker::auth_client()?;
    authenticate(&mut client).await?;

    Ok(())
}

async fn authenticate(client: &mut Client) -> anyhow::Result<AuthInfo> {
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
        authorization_url = response.verification_uri_complete().map(|url| url.secret()).unwrap_or_else(|| response.verification_uri())
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

    ht_update_checker::save_auth_info(&auth_info)
        .await
        .context("Unable to save new authentication info")?;

    log::info!("Success!");

    Ok(auth_info)
}
