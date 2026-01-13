use crate::auth::{AccessTokenInfo, AuthInfo};
use crate::util::BetterJsonError;
use anyhow::{Context, bail};
use chrono::{DateTime, Duration, Utc};
use oauth2::basic::{
    BasicClient, BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
    BasicTokenResponse,
};
use oauth2::{
    AuthType, AuthUrl, ClientId, DeviceAuthorizationUrl, EndpointNotSet, EndpointSet, RedirectUrl,
    RefreshToken, StandardRevocableToken, TokenResponse, TokenUrl,
};
use reqwest::header::{ACCEPT_ENCODING, AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, IntoUrl, Method, RequestBuilder, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Default, Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    #[default]
    Release,
    PreRelease,
    #[serde(untagged)]
    Unknown(String),
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum LauncherUpdateChannel {
    #[default]
    Release,
    Stage,
    #[serde(untagged)]
    Unknown(String),
}

impl UpdateChannel {
    pub fn values() -> [Self; 2] {
        [UpdateChannel::Release, UpdateChannel::PreRelease]
    }

    pub fn color(&self) -> u32 {
        match self {
            UpdateChannel::Release => 0x476ed1,
            UpdateChannel::PreRelease => 0xb22828,
            UpdateChannel::Unknown(_) => 0xb22828,
        }
    }
}

impl Display for UpdateChannel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                UpdateChannel::Release => "release",
                UpdateChannel::PreRelease => "pre-release",
                UpdateChannel::Unknown(value) => value,
            }
        )
    }
}

impl FromStr for UpdateChannel {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        for ch in UpdateChannel::values() {
            if ch.to_string() == value {
                return Ok(ch);
            }
        }

        Ok(UpdateChannel::Unknown(value.to_string()))
    }
}

impl LauncherUpdateChannel {
    pub fn values() -> [Self; 2] {
        [LauncherUpdateChannel::Release, LauncherUpdateChannel::Stage]
    }
}

impl Display for LauncherUpdateChannel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LauncherUpdateChannel::Release => "release",
                LauncherUpdateChannel::Stage => "stage",
                LauncherUpdateChannel::Unknown(value) => value,
            }
        )
    }
}

impl FromStr for LauncherUpdateChannel {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        for ch in LauncherUpdateChannel::values() {
            if ch.to_string() == value {
                return Ok(ch);
            }
        }

        Ok(LauncherUpdateChannel::Unknown(value.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct LauncherMeta {
    pub channel: LauncherUpdateChannel,
    pub version: String,
    pub downloads: DownloadMeta,
}

impl LauncherMeta {
    fn from_update_meta(value: UpdateMeta, channel: &LauncherUpdateChannel) -> Self {
        LauncherMeta {
            channel: channel.clone(),
            version: value.version.clone(),
            downloads: value.get_downloads(),
        }
    }

    fn as_header_map(&self) -> anyhow::Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.append(
            HeaderName::from_static("x-hytale-launcher-branch"),
            HeaderValue::from_maybe_shared(self.channel.to_string())?,
        );
        headers.append(
            HeaderName::from_static("x-hytale-launcher-version"),
            HeaderValue::from_maybe_shared(self.version.clone())?,
        );

        Ok(headers)
    }
}

pub struct JavaVersionMeta {
    pub channel: UpdateChannel,
    pub version: String,
    pub downloads: DownloadMeta,
}

impl JavaVersionMeta {
    fn from_update_meta(value: UpdateMeta, channel: &UpdateChannel) -> Self {
        JavaVersionMeta {
            channel: channel.clone(),
            version: value.version.clone(),
            downloads: value.get_downloads(),
        }
    }
}

#[derive(Deserialize, Debug)]
struct UpdateMeta {
    version: String,

    #[serde(rename = "download_url")]
    downloads: HashMap<TargetOs, HashMap<TargetArchitecture, DownloadInfo>>,
}

type DownloadMeta = HashMap<(TargetOs, TargetArchitecture), DownloadInfo>;

impl UpdateMeta {
    fn get_downloads(&self) -> HashMap<(TargetOs, TargetArchitecture), DownloadInfo> {
        self.downloads
            .iter()
            .flat_map(|(os, map)| {
                map.iter()
                    .map(|(arch, info)| ((os.clone(), arch.clone()), info.clone()))
            })
            .collect()
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct DownloadInfo {
    pub url: String,
    pub sha256: String,
}

#[derive(Deserialize, Debug, Clone, Hash, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TargetOs {
    Windows,
    Linux,
    #[serde(rename = "darwin")]
    MacOS,
    #[serde(untagged)]
    Unknown(String),
}

impl Display for TargetOs {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TargetOs::Windows => "windows",
                TargetOs::Linux => "linux",
                TargetOs::MacOS => "darwin",
                TargetOs::Unknown(value) => value,
            }
        )
    }
}

#[derive(Deserialize, Debug, Clone, Hash, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TargetArchitecture {
    Amd64,
    Arm64,
    #[serde(untagged)]
    Unknown(String),
}

impl Display for TargetArchitecture {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TargetArchitecture::Amd64 => "amd64",
                TargetArchitecture::Arm64 => "arm64",
                TargetArchitecture::Unknown(value) => value,
            }
        )
    }
}

pub async fn get_launcher_meta(
    client: &Client,
    update_channel: &LauncherUpdateChannel,
) -> anyhow::Result<LauncherMeta> {
    let url = format!(
        "https://launcher.arcanitegames.ca/version/{update_channel}/launcher.json",
        update_channel = "stage"
    ); // FIXME temp fix
    Ok(LauncherMeta::from_update_meta(
        get_update_meta(client, &url, None).await?,
        update_channel,
    ))
}

pub async fn get_java_version_meta(
    client: &Client,
    update_channel: &UpdateChannel,
    launcher_meta: &LauncherMeta,
) -> anyhow::Result<JavaVersionMeta> {
    let url = format!("https://launcher.arcanitegames.ca/version/{update_channel}/jre.json");

    let headers = launcher_meta.as_header_map()?;
    let meta = get_update_meta(client, &url, Some(headers)).await?;
    Ok(JavaVersionMeta::from_update_meta(meta, update_channel))
}

async fn get_update_meta(
    client: &Client,
    url: impl IntoUrl,
    extra_headers: Option<HeaderMap>,
) -> anyhow::Result<UpdateMeta> {
    let mut req = client.get(url);
    if let Some(extra_headers) = extra_headers {
        req = req.headers(extra_headers);
    }

    make_request(req)
        .await?
        .json_with_error::<UpdateMeta>()
        .await
}

#[derive(Deserialize, Debug, Clone)]
pub struct AccountMetaData {
    pub owner: Uuid,
    pub patchlines: Option<HashMap<UpdateChannel, Patchline>>,
    pub profiles: Vec<AccountProfile>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Patchline {
    pub build_version: String,
    #[serde(rename = "newest")]
    pub current_build: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountProfile {
    pub created_at: DateTime<Utc>,
    pub next_name_change_at: Option<DateTime<Utc>>,
    pub entitlements: Vec<String>,
    pub username: String,
    pub uuid: Uuid,
}

pub async fn get_account_meta(
    client: &mut ArcaniteClient,
    launcher_meta: &LauncherMeta,
    operating_system: TargetOs,
    architecture: TargetArchitecture,
) -> anyhow::Result<AccountMetaData> {
    let url = format!(
        "https://account-data.arcanitegames.ca/my-account/get-launcher-data?os={operating_system}&arch={architecture}"
    );
    let headers = launcher_meta.as_header_map()?;

    let req = client
        .get(url)
        .await?
        .headers(headers)
        .header(ACCEPT_ENCODING, HeaderValue::from_static("gzip"));

    make_request(req)
        .await?
        .json_with_error::<AccountMetaData>()
        .await
}

#[derive(Deserialize, Debug, Clone)]
pub struct HytaleDownloaderMeta {
    #[serde(rename = "latest")]
    pub latest_version: String,
}

pub async fn get_downloader_meta(
    client: &mut ArcaniteClient,
) -> anyhow::Result<HytaleDownloaderMeta> {
    const URL: &str = "https://downloader.arcanitegames.ca/version.json";

    let req = client
        .http_client
        .get(URL)
        .header(ACCEPT_ENCODING, HeaderValue::from_static("gzip"));

    make_request(req)
        .await?
        .json_with_error::<HytaleDownloaderMeta>()
        .await
}

pub struct ArcaniteClient {
    auth_info: AuthInfo,
    http_client: Client,
}

impl ArcaniteClient {
    pub fn new(http_client: Client, auth_info: AuthInfo) -> Self {
        Self {
            auth_info,
            http_client,
        }
    }

    pub async fn request(
        &mut self,
        method: Method,
        url: impl IntoUrl,
    ) -> anyhow::Result<RequestBuilder> {
        let auth_token = self
            .check_auth()
            .await
            .context("unable to validate auth token")?;

        let builder = self.http_client.request(method, url).header(
            AUTHORIZATION,
            HeaderValue::from_maybe_shared(format!(
                "{token_type} {token}",
                token_type = auth_token.token_type,
                token = auth_token.token
            ))?,
        );

        Ok(builder)
    }

    pub async fn get(&mut self, url: impl IntoUrl) -> anyhow::Result<RequestBuilder> {
        self.request(Method::GET, url).await
    }

    pub async fn post(&mut self, url: impl IntoUrl) -> anyhow::Result<RequestBuilder> {
        self.request(Method::POST, url).await
    }

    async fn check_auth(&mut self) -> anyhow::Result<AccessTokenInfo> {
        const GRACE_TIME: i64 = 32;

        if self.auth_info.client_id != OAUTH_CLIENT_ID {
            bail!("Saved client ID does not match, please reauthenticate!");
        }

        if let Some(current_token) = self.auth_info.auth_token.clone()
            && current_token.expires_at > Utc::now() + Duration::seconds(GRACE_TIME)
        {
            return Ok(current_token);
        }

        log::debug!("Refreshing auth...");
        let auth_http_client = crate::auth_client()?;

        let refresh_token = RefreshToken::new(self.auth_info.refresh_token.clone());

        let now = Utc::now();
        let auth_client = oauth_client();
        let result = auth_client
            .exchange_refresh_token(&refresh_token)
            .request_async(&auth_http_client)
            .await
            .context("Unable to refresh auth token")?;

        let Some(new_refresh_token) = result.refresh_token() else {
            bail!("No refresh token provided in auth response!")
        };
        let expires_at = now
            + Duration::seconds(
                result
                    .expires_in()
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(3600),
            );

        let auth_info = AccessTokenInfo {
            token: result.access_token().secret().clone(),
            token_type: result.token_type().into(),
            expires_at,
        };

        self.auth_info = AuthInfo {
            auth_token: Some(auth_info.clone()),
            refresh_token: new_refresh_token.secret().clone(),
            scopes: self.auth_info.scopes.clone(),
            client_id: self.auth_info.client_id.clone(),
        };
        crate::save_auth_info(&self.auth_info)
            .await
            .context("Unable to save new authentication info")?;

        log::debug!(
            "Success! new auth token expires at: {exp}",
            exp = auth_info.expires_at
        );
        Ok(auth_info)
    }
}

async fn make_request(builder: RequestBuilder) -> anyhow::Result<Response> {
    let response = builder.send().await?;

    let status = response.status();
    if !status.is_success() {
        let url = response.url().clone();
        let error_message = response.text().await.ok().unwrap_or_else(|| {
            status
                .canonical_reason()
                .unwrap_or("UNKNOWN REASON")
                .to_string()
        });
        bail!(
            "{url} Received HTTP Status {status_code} {error_message}",
            status_code = status.as_str()
        );
    }

    Ok(response)
}

pub type OAuthClient = oauth2::Client<
    BasicErrorResponse,
    BasicTokenResponse,
    BasicTokenIntrospectionResponse,
    StandardRevocableToken,
    BasicRevocationErrorResponse,
    EndpointSet,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet,
>;

// hytale-server
// hytale-launcher
// hytale-downloader
pub const OAUTH_CLIENT_ID: &str = "hytale-launcher";

pub fn oauth_client() -> OAuthClient {
    let auth_url = AuthUrl::new("https://oauth.accounts.arcanitegames.ca/oauth2/auth".to_string())
        .expect("Invalid Auth URL");
    let token_url =
        TokenUrl::new("https://oauth.accounts.arcanitegames.ca/oauth2/token".to_string())
            .expect("Invalid Token URL");
    let redirect_uri =
        RedirectUrl::new("https://accounts.arcanitegames.ca/consent/client".to_string())
            .expect("Invalid Redirect URI");
    let device_auth_url = DeviceAuthorizationUrl::new(
        "https://oauth.accounts.arcanitegames.ca/oauth2/device/auth".to_string(),
    )
    .expect("invalid Device Auth URL");

    let client_id = ClientId::new(OAUTH_CLIENT_ID.to_string());

    BasicClient::new(client_id)
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(redirect_uri)
        .set_device_authorization_url(device_auth_url)
        .set_auth_type(AuthType::RequestBody)
}
