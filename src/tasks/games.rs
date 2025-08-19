use std::time::Duration;

use modio::request::filter::prelude::*;
use tokio::time::{self, Instant};
use tracing::error;

use crate::bot::Context;
use crate::db::autocomplete::{replace_games, Game};
use crate::db::types::{ApiAccessOptions, GameId};
use crate::error::Error;
use crate::Page;

const MIN: Duration = Duration::from_secs(45);
const INTERVAL_DURATION: Duration = Duration::from_secs(4500);

pub async fn task(ctx: Context) {
    let mut interval = time::interval_at(Instant::now() + MIN, INTERVAL_DURATION);

    loop {
        interval.tick().await;

        let pool = ctx.pool.clone();
        let modio = ctx.modio.clone();

        let task = async move {
            let first_page = modio.get_games().filter(Id::asc()).await?.data().await?;

            let mut games = first_page
                .data
                .into_iter()
                .map(|g| Game {
                    id: GameId(g.id),
                    name: g.name,
                    name_id: g.name_id,
                    api_access_options: ApiAccessOptions(g.api_access_options),
                })
                .collect::<Vec<_>>();

            loop {
                let filter = Id::asc().offset((first_page.offset + first_page.limit) as usize);

                let next_page = modio.get_games().filter(filter).await?.data().await?;

                if next_page.is_empty() {
                    break;
                }

                games.extend(next_page.data.into_iter().map(|g| Game {
                    id: GameId(g.id),
                    name: g.name,
                    name_id: g.name_id,
                    api_access_options: ApiAccessOptions(g.api_access_options),
                }));
            }

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
