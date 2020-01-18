use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use dbl::{types::ShardStats, Client};
use futures::{future, StreamExt};
use log::error;
use serenity::cache::CacheRwLock;
use tokio::runtime::Handle;
use tokio::time::{interval_at, Instant};

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
    cache: CacheRwLock,
    token: &str,
    handle: Handle,
) -> Result<impl Future<Output = ()>, Error> {
    let bot = get_bot_id(bot);
    let client = Arc::new(Client::new(token.to_owned()).map_err(Error::Dbl)?);

    let task = interval_at(Instant::now() + MIN, SIX_HOURS).for_each(move |_| {
        let client = Arc::clone(&client);
        let servers = cache.read().guilds.len();
        let stats = ShardStats::Cumulative {
            server_count: servers as u64,
            shard_count: None,
        };

        handle.spawn(async move {
            match client.update_stats(bot, stats).await {
                Ok(_) => log::info!("Update bot stats [servers={}]", servers),
                Err(e) => error!("Failed to update bot stats: {:?}", e),
            }
        });
        future::ready(())
    });

    Ok(task)
}

// vim: fdm=marker
