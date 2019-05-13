use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};

use futures::future::{self, Either};
use futures::{Future, Stream};
use log::error;
use reqwest::header::{HeaderMap, HeaderValue, InvalidHeaderValue, AUTHORIZATION};
use reqwest::r#async::Client;
use reqwest::Error as ReqwestError;
use serenity::CACHE;
use tokio::runtime::TaskExecutor;
use tokio::timer::Interval;

use crate::util;

const DBL_BASE_URL: &str = "https://discordbots.org/bot";
const DBL: &str = "https://discordbots.org/api/bots";
const MIN: Duration = Duration::from_secs(1 * 60);
const SIX_HOURS: Duration = Duration::from_secs(6 * 60 * 60);

struct DblClient {
    client: Client,
}

impl DblClient {
    fn new(token: &str) -> Result<DblClient, Error> {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(token)?);

        let client = Client::builder().default_headers(headers).build()?;

        Ok(DblClient { client })
    }

    fn update_stats(&self, bot: u64, servers: usize) -> impl Future<Item = (), Error = Error> {
        let mut data = HashMap::new();
        data.insert("server_count", servers);

        self.client
            .post(&format!("{}/{}/stats", DBL, bot))
            .json(&data)
            .send()
            .and_then(move |r| {
                if r.status().is_success() {
                    log::info!("Update bot stats [servers={}]", servers);

                    Either::A(future::ok(()))
                } else {
                    Either::B(r.into_body().concat2().and_then(move |body| {
                        if let Ok(s) = std::str::from_utf8(&body) {
                            log::error!("Failed to update bot stats: {}", s);
                        } else {
                            log::error!("Failed to update bot stats");
                        }
                        Ok(())
                    }))
                }
            })
            .map_err(Error::from)
    }
}

#[derive(Debug)]
pub enum Error {
    Http(InvalidHeaderValue),
    Client(ReqwestError),
}

// impl StdError, Display, From for Error {{{
impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Client(e) => e.fmt(f),
            Error::Http(e) => e.fmt(f),
        }
    }
}

impl From<InvalidHeaderValue> for Error {
    fn from(e: InvalidHeaderValue) -> Error {
        Error::Http(e)
    }
}

impl From<ReqwestError> for Error {
    fn from(e: ReqwestError) -> Error {
        Error::Client(e)
    }
}
// }}}

pub fn is_dbl_enabled() -> bool {
    util::var(crate::DBL_TOKEN).is_ok()
}

pub fn get_bot_id() -> u64 {
    util::var(crate::DBL_OVERRIDE_BOT_ID)
        .ok()
        .and_then(|id| id.parse::<u64>().ok())
        .unwrap_or_else(|| *CACHE.read().user.id.as_u64())
}

pub fn get_profile() -> String {
    format!("{}/{}", DBL_BASE_URL, get_bot_id())
}

pub fn task(
    token: &str,
    executor: TaskExecutor,
) -> Result<impl Future<Item = (), Error = ()>, Error> {
    let client = DblClient::new(token)?;

    Ok(Interval::new(Instant::now() + MIN, SIX_HOURS)
        .for_each(move |_| {
            let bot = get_bot_id();
            let servers = CACHE.read().guilds.len();
            let task = client
                .update_stats(bot, servers)
                .map_err(|e| error!("Failed to update bot stats: {:?}", e));

            executor.spawn(task);
            Ok(())
        })
        .map_err(|e| error!("Interval errored: {}", e)))
}

// vim: fdm=marker
