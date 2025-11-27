use std::fmt;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;

use serde::de::{Deserialize, Deserializer, Error, Visitor};
use serde_derive::Deserialize;

use crate::Result;

const DEFAULT_METRICS_SOCKET_ADDR: ([u8; 4], u16) = ([127, 0, 0, 1], 8080);

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
    pub database_url: String,
}

#[derive(Deserialize)]
pub struct ModioConfig {
    pub host: Option<Host>,
    pub api_key: String,
    pub token: Option<String>,
}

#[derive(Debug)]
pub enum Host {
    Default,
    Test,
    Dynamic,
    DynamicWithCustom(String),
    Custom(String),
}

impl<'de> Deserialize<'de> for Host {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct StrVisitor;

        impl Visitor<'_> for StrVisitor {
            type Value = Host;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("string")
            }

            fn visit_str<E: Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
                let host = match v {
                    "default" => Host::Default,
                    "test" => Host::Test,
                    "dynamic" => Host::Dynamic,
                    _ => {
                        if let Some(host) = v.strip_prefix("dynamic:") {
                            Host::DynamicWithCustom(host.to_owned())
                        } else {
                            Host::Custom(v.to_owned())
                        }
                    }
                };

                Ok(host)
            }
        }

        deserializer.deserialize_str(StrVisitor)
    }
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

fn default_metrics_socket_addr() -> SocketAddr {
    SocketAddr::from(DEFAULT_METRICS_SOCKET_ADDR)
}
