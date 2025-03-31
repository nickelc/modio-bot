use std::time::Duration;

use modio::request::filter::prelude::*;
use modio::util::Paginate;
use tokio::time::{self, Instant};
use tracing::error;

use crate::bot::Context;
use crate::db::autocomplete::{replace_games, Game};
use crate::db::types::{ApiAccessOptions, GameId};
use crate::error::Error;

const MIN: Duration = Duration::from_secs(45);
const INTERVAL_DURATION: Duration = Duration::from_secs(4500);

pub async fn task(ctx: Context) {
    let mut interval = time::interval_at(Instant::now() + MIN, INTERVAL_DURATION);

    loop {
        interval.tick().await;

        let pool = ctx.pool.clone();
        let modio = ctx.modio.clone();

        let task = async move {
            let games = {
                let games = modio.get_games().filter(Id::asc());
                let mut paged = games.paged();

                let mut games = Vec::new();
                while let Some(page) = paged.next().await? {
                    for g in page {
                        games.push(Game {
                            id: GameId(g.id),
                            name: g.name,
                            name_id: g.name_id,
                            api_access_options: ApiAccessOptions(g.api_access_options),
                        });
                    }
                }
                games
            };

            if let Err(e) = replace_games(&pool, &games) {
                error!("{e}");
            }

            Ok::<_, Error>(())
        };

        tokio::spawn(async {
            if let Err(e) = task.await {
                error!("{e}");
            }
        });
    }
}
