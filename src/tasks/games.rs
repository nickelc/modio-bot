use std::time::Duration;

use modio::filter::prelude::*;
use tokio::time::{self, Instant};
use tracing::error;

use crate::bot::Context;
use crate::db::autocomplete::{replace_games, Game};
use crate::db::types::{ApiAccessOptions, GameId};

const MIN: Duration = Duration::from_secs(45);
const INTERVAL_DURATION: Duration = Duration::from_secs(4500);

pub async fn task(ctx: Context) {
    let mut interval = time::interval_at(Instant::now() + MIN, INTERVAL_DURATION);

    loop {
        interval.tick().await;

        let pool = ctx.pool.clone();
        let games = ctx.modio.games().search(Id::asc());

        let task = async move {
            let games = games.collect().await?;
            let games = games
                .iter()
                .map(|g| Game {
                    id: GameId(g.id),
                    name: &g.name,
                    name_id: &g.name_id,
                    api_access_options: ApiAccessOptions(g.api_access_options),
                })
                .collect::<Vec<_>>();

            if let Err(e) = replace_games(&pool, &games) {
                error!("{e}");
            }

            Ok::<_, modio::Error>(())
        };

        tokio::spawn(async {
            if let Err(e) = task.await {
                error!("{e}");
            }
        });
    }
}
