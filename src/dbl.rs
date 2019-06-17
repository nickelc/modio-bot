use std::sync::Arc;
use std::time::{Duration, Instant};

use dbl::{types::ShardStats, Client};
use futures::{Future, Stream};
use log::error;
use serenity::cache::Cache;
use serenity::prelude::*;
use tokio::runtime::TaskExecutor;
use tokio::timer::Interval;

use crate::error::Error;
use crate::util;

const DBL_BASE_URL: &str = "https://discordbots.org/bot";
const MIN: Duration = Duration::from_secs(60);
const SIX_HOURS: Duration = Duration::from_secs(6 * 60 * 60);

pub fn is_dbl_enabled() -> bool {
    util::var(crate::DBL_TOKEN).is_ok()
}

fn get_bot_id(bot: u64) -> u64 {
    util::var(crate::DBL_OVERRIDE_BOT_ID)
        .ok()
        .and_then(|id| id.parse::<u64>().ok())
        .unwrap_or(bot)
}

pub fn get_profile(bot: u64) -> String {
    format!("{}/{}", DBL_BASE_URL, get_bot_id(bot))
}

pub fn task(
    bot: u64,
    cache: Arc<RwLock<Cache>>,
    token: &str,
    executor: TaskExecutor,
) -> Result<impl Future<Item = (), Error = ()>, Error> {
    let bot = get_bot_id(bot);
    let client = Client::new(token.to_owned()).map_err(Error::Dbl)?;

    Ok(Interval::new(Instant::now() + MIN, SIX_HOURS)
        .for_each(move |_| {
            let servers = cache.read().guilds.len();
            let stats = ShardStats::Cumulative {
                server_count: servers as u64,
                shard_count: None,
            };

            let task = client
                .update_stats(bot, stats)
                .map(move |_| {
                    log::info!("Update bot stats [servers={}]", servers);
                })
                .map_err(|e| error!("Failed to update bot stats: {:?}", e));

            executor.spawn(task);
            Ok(())
        })
        .map_err(|e| error!("Interval errored: {}", e)))
}

// vim: fdm=marker
