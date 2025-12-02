use crate::auth::AuthInfo;
use crate::util::BetterJsonError;
use anyhow::bail;
use chrono::{DateTime, Utc};
use reqwest::header::{ACCEPT_ENCODING, AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, IntoUrl};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Default, Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    #[default]
    Release,
    #[serde(untagged)]
    Unknown(String),
}

impl UpdateChannel {
    pub fn values() -> [Self; 1] {
        [UpdateChannel::Release]
    }

    pub fn color(&self) -> u32 {
        match self {
            UpdateChannel::Release => 0x476ed1,
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
                UpdateChannel::Unknown(value) => value,
            }
        )
    }
}

impl FromStr for UpdateChannel {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        for ch in UpdateChannel::values() {
            if ch.to_string() == value {
                return Ok(ch);
            }
        }

        Err(format!("Unknown channel: {value}"))
    }
}

#[derive(Debug, Clone)]
pub struct LauncherMeta {
    pub channel: UpdateChannel,
    pub version: String,
    pub downloads: DownloadMeta,
}

impl LauncherMeta {
    fn from_update_meta(value: UpdateMeta, channel: &UpdateChannel) -> Self {
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
    fn from_update_meta(value: UpdateMeta, launcher_meta: &LauncherMeta) -> Self {
        JavaVersionMeta {
            channel: launcher_meta.channel.clone(),
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
    update_channel: &UpdateChannel,
) -> anyhow::Result<LauncherMeta> {
    let url = format!("https://launcher.arcanitegames.ca/version/{update_channel}/launcher.json");
    Ok(LauncherMeta::from_update_meta(
        get_update_meta(client, &url, None).await?,
        update_channel,
    ))
}

pub async fn get_java_version_meta(
    client: &Client,
    launcher_meta: &LauncherMeta,
) -> anyhow::Result<JavaVersionMeta> {
    let url = format!(
        "https://launcher.arcanitegames.ca/version/{update_channel}/jre.json",
        update_channel = launcher_meta.channel
    );

    let headers = launcher_meta.as_header_map()?;
    let meta = get_update_meta(client, &url, Some(headers)).await?;
    Ok(JavaVersionMeta::from_update_meta(meta, launcher_meta))
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
    let response = req.send().await?;

    let status = response.status();
    if !status.is_success() {
        let error_message = response.text().await.ok().unwrap_or_else(|| {
            status
                .canonical_reason()
                .unwrap_or("UNKNOWN REASON")
                .to_string()
        });
        bail!(
            "Received HTTP Status {status_code} {error_message}",
            status_code = status.as_str()
        );
    }

    response.json_with_error::<UpdateMeta>().await
}

#[derive(Deserialize, Debug, Clone)]
pub struct AccountMetaData {
    pub owner: Uuid,
    pub patchlines: HashMap<UpdateChannel, Patchline>,
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
    client: &Client,
    auth_info: &AuthInfo,
    launcher_meta: &LauncherMeta,
    operating_system: TargetOs,
    architecture: TargetArchitecture,
) -> anyhow::Result<AccountMetaData> {
    let url = format!(
        "https://account-data.arcanitegames.ca/my-account/get-launcher-data?os={operating_system}&arch={architecture}"
    );
    let headers = launcher_meta.as_header_map()?;

    let Some(access_token_info) = auth_info.auth_token.clone() else {
        bail!("No auth token present!");
    };

    let response = client
        .get(url)
        .headers(headers)
        .header(
            AUTHORIZATION,
            format!(
                "{token_type} {token}",
                token_type = access_token_info.token_type,
                token = access_token_info.token
            ),
        )
        .header(ACCEPT_ENCODING, HeaderValue::from_static("gzip"))
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_message = response.text().await.ok().unwrap_or_else(|| {
            status
                .canonical_reason()
                .unwrap_or("UNKNOWN REASON")
                .to_string()
        });
        bail!(
            "Received HTTP Status {status_code} {error_message}",
            status_code = status.as_str()
        );
    }

    response.json_with_error::<AccountMetaData>().await
}
