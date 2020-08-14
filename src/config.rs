use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::result::Result as StdResult;

use http::Uri;
use serde::Deserialize;
use url::Url;

use crate::Result;

pub const DBL_OVERRIDE_BOT_ID: &str = "DBL_OVERRIDE_BOT_ID";

const DEFAULT_METRICS_SOCKET_ADDR: ([u8; 4], u16) = ([127, 0, 0, 1], 8080);
const DEFAULT_OAUTH_SOCKET_ADDR: ([u8; 4], u16) = ([127, 0, 0, 1], 8000);
const DEFAULT_AUTH_URL: &str = "https://discord.com/api/v6/oauth2/authorize";
const DEFAULT_TOKEN_URL: &str = "https://discord.com/api/v6/oauth2/token";

const DEFAULT_MODIO_HOST: &str = "https://api.mod.io/v1";

#[derive(Deserialize)]
pub struct Config {
    pub bot: BotConfig,
    pub modio: ModioConfig,
    #[serde(default)]
    pub metrics: MetricsConfig,
}

#[derive(Deserialize)]
pub struct MetricsConfig {
    #[serde(default = "default_metrics_socket_addr")]
    pub addr: SocketAddr,
}

#[derive(Deserialize)]
pub struct BotConfig {
    pub token: String,
    pub dbl_token: Option<String>,
    pub database_url: String,
    pub oauth: OAuthConfig,
}

#[derive(Deserialize)]
pub struct OAuthConfig {
    #[serde(default = "default_oauth_socket_addr")]
    pub addr: SocketAddr,
    pub client_id: String,
    pub client_secret: String,
    #[serde(default = "default_auth_url")]
    pub auth_url: Url,
    #[serde(default = "default_token_url")]
    pub token_url: Url,
    pub redirect_uri: Url,
    pub login_url: Url,
    #[serde(default = "default_location_after_login")]
    #[serde(deserialize_with = "deserialize_uri")]
    pub location_after_login: Uri,
}

#[derive(Deserialize)]
pub struct ModioConfig {
    #[serde(default = "default_modio_host")]
    pub host: String,
    pub api_key: String,
    pub token: Option<String>,
}

pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Config> {
    let data = fs::read_to_string(path)?;
    Ok(toml::from_str(&data)?)
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            addr: default_metrics_socket_addr(),
        }
    }
}

fn deserialize_uri<'de, D>(deserializer: D) -> StdResult<Uri, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{Error, Unexpected, Visitor};
    use std::fmt;
    use std::str::FromStr;

    struct UriVisitor;

    impl<'de> Visitor<'de> for UriVisitor {
        type Value = Uri;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a string representing an URI")
        }

        fn visit_str<E>(self, s: &str) -> StdResult<Self::Value, E>
        where
            E: Error,
        {
            Uri::from_str(s)
                .map_err(|e| Error::invalid_value(Unexpected::Str(s), &e.to_string().as_str()))
        }
    }

    deserializer.deserialize_str(UriVisitor)
}

fn default_metrics_socket_addr() -> SocketAddr {
    SocketAddr::from(DEFAULT_METRICS_SOCKET_ADDR)
}

fn default_modio_host() -> String {
    DEFAULT_MODIO_HOST.to_owned()
}

fn default_oauth_socket_addr() -> SocketAddr {
    SocketAddr::from(DEFAULT_OAUTH_SOCKET_ADDR)
}

fn default_location_after_login() -> Uri {
    Uri::from_static("/login")
}

fn default_auth_url() -> Url {
    Url::parse(DEFAULT_AUTH_URL).unwrap()
}

fn default_token_url() -> Url {
    Url::parse(DEFAULT_TOKEN_URL).unwrap()
}
