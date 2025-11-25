use chrono::{DateTime, Utc};
use oauth2::basic::BasicTokenType;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AuthInfo {
    pub auth_token: Option<AccessTokenInfo>,
    pub refresh_token: String,
    pub scopes: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub enum AuthTokenType {
    #[default]
    Bearer,
    Mac,
    #[serde(untagged)]
    Extension(String),
}

impl Display for AuthTokenType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AuthTokenType::Bearer => "Bearer",
                AuthTokenType::Mac => "Mac",
                AuthTokenType::Extension(value) => value,
            }
        )
    }
}

impl From<&BasicTokenType> for AuthTokenType {
    fn from(value: &BasicTokenType) -> Self {
        match value {
            BasicTokenType::Bearer => AuthTokenType::Bearer,
            BasicTokenType::Mac => AuthTokenType::Mac,
            BasicTokenType::Extension(str) => AuthTokenType::Extension(str.clone()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccessTokenInfo {
    pub token: String,
    pub token_type: AuthTokenType,
    pub expires_at: DateTime<Utc>,
}
