use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use dbl::{types::ShardStats, Client};
use serenity::cache::Cache;
use tokio::time::{interval_at, Instant};
use tracing::{error, info};

use crate::config::DBL_OVERRIDE_BOT_ID;
use crate::error::Error;

const MIN: Duration = Duration::from_secs(60);
const SIX_HOURS: Duration = Duration::from_secs(6 * 60 * 60);

fn get_bot_id(bot: u64) -> u64 {
    std::env::var(DBL_OVERRIDE_BOT_ID)
        .ok()
        .and_then(|id| id.parse::<u64>().ok())
        .unwrap_or(bot)
}

pub fn task(bot: u64, cache: Arc<Cache>, token: &str) -> Result<impl Future<Output = ()>, Error> {
    let bot = get_bot_id(bot);
    let client = Arc::new(Client::new(token.to_owned()).map_err(Error::Dbl)?);

    Ok(async move {
        let mut interval = interval_at(Instant::now() + MIN, SIX_HOURS);
        loop {
            interval.tick().await;
            let client = Arc::clone(&client);
            let servers = cache.guild_count().await;
            let stats = ShardStats::Cumulative {
                server_count: servers as u64,
                shard_count: None,
            };

            tokio::spawn(async move {
                match client.update_stats(bot, stats).await {
                    Ok(_) => info!("Update bot stats [servers={}]", servers),
                    Err(e) => error!("Failed to update bot stats: {:?}", e),
                }
            });
        }
    })
}

// vim: fdm=marker
