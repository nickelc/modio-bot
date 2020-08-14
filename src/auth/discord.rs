use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize};
use serenity::model::id::UserId;
use url::Url;

const USER_API: &str = "https://discord.com/api/v6/users/@me";

pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: Url,
    pub token_url: Url,
    pub redirect_uri: Url,
    pub scope: &'static str,
}

impl OAuthConfig {
    pub fn authorize_url(&self) -> Url {
        let mut url = self.auth_url.clone();
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("client_id", &self.client_id);
            query.append_pair("response_type", "code");
            query.append_pair("redirect_uri", self.redirect_uri.as_str());
            query.append_pair("scope", self.scope);
        }
        url
    }
}

#[derive(Deserialize)]
pub struct CurrentUser {
    pub id: UserId,
    #[serde(rename = "username")]
    pub name: String,
    pub discriminator: String,
}

#[derive(Serialize)]
struct TokenRequest<'a> {
    client_id: &'a str,
    client_secret: &'a str,
    grant_type: &'static str,
    code: String,
    redirect_uri: &'a Url,
    scope: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct Token {
    pub access_token: String,
    pub token_type: String,
    #[serde(deserialize_with = "deserialize_seconds")]
    pub expires_in: Duration,
    pub refresh_token: String,
    pub scope: String,
}

#[derive(Clone)]
pub struct Client {
    inner: reqwest::Client,
    config: Arc<OAuthConfig>,
}

impl Client {
    pub fn new(config: Arc<OAuthConfig>) -> Client {
        Client {
            inner: reqwest::Client::new(),
            config,
        }
    }

    pub async fn request_token(&self, code: String) -> Result<Token, Error> {
        let url = self.config.token_url.clone();
        let data = TokenRequest {
            client_id: &self.config.client_id,
            client_secret: &self.config.client_secret,
            grant_type: "authorization_code",
            code,
            redirect_uri: &self.config.redirect_uri,
            scope: self.config.scope,
        };

        let resp = self.inner.post(url).form(&data).send().await?;

        if resp.status().is_success() {
            Ok(resp.json().await?)
        } else {
            let err = resp.json::<OAuthError>().await?;
            Err(Error::OAuth(err))
        }
    }

    pub async fn current_user(&self, token: &str) -> Result<CurrentUser, Error> {
        let resp = self.inner.get(USER_API).bearer_auth(token).send().await?;
        Ok(resp.error_for_status()?.json().await?)
    }
}

#[derive(Debug)]
pub enum Error {
    Client(reqwest::Error),
    OAuth(OAuthError),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Error {
        Error::Client(e)
    }
}

#[derive(Debug, Deserialize)]
pub struct OAuthError {
    pub error: ErrorKind,
    #[serde(rename = "error_description")]
    pub description: Option<String>,
    #[serde(rename = "error_uri")]
    pub uri: Option<String>,
}

impl fmt::Display for OAuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = self.error.to_string();
        if let Some(desc) = &self.description {
            s.push_str(": ");
            s.push_str(desc);
        }
        if let Some(uri) = &self.uri {
            s.push_str(" / See ");
            s.push_str(uri);
        }
        write!(f, "{}", s)
    }
}

impl std::error::Error for OAuthError {}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    InvalidRequest,
    InvalidClient,
    InvalidGrant,
    UnauthorizedClient,
    UnsupportedGrantType,
    InvalidScope,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::InvalidRequest => f.write_str("invalid_request"),
            ErrorKind::InvalidClient => f.write_str("invalid_client"),
            ErrorKind::InvalidGrant => f.write_str("invalid_grant"),
            ErrorKind::UnauthorizedClient => f.write_str("unauthorized_client"),
            ErrorKind::UnsupportedGrantType => f.write_str("unsupported_grant_type"),
            ErrorKind::InvalidScope => f.write_str("invalid_scope"),
        }
    }
}

fn deserialize_seconds<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    u64::deserialize(deserializer).map(Duration::from_secs)
}
