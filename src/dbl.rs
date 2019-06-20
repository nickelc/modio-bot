use std::time::{Duration, Instant};

use dbl::{types::ShardStats, Client};
use futures::{Future, Stream};
use log::error;
use serenity::CACHE;
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
    let client = Client::new(token.to_owned()).map_err(Error::Dbl)?;

    Ok(Interval::new(Instant::now() + MIN, SIX_HOURS)
        .for_each(move |_| {
            let bot = get_bot_id();
            let servers = CACHE.read().guilds.len();
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
