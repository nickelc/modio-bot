use std::time::Duration;

use futures::{Future, Stream};
use log::{debug, warn};
use modio::filter::{Operator, Order};
use modio::games::GamesListOptions;
use modio::EventListOptions;
use modio::Modio;
use serenity::prelude::*;
use tokio::runtime::TaskExecutor;
use tokio::timer::Interval;

use crate::db::Subscriptions;
use crate::util;

command!(
    Subscribe(self, ctx, msg, args) {
        let mut ctx2 = ctx.clone();
        let channel_id = msg.channel_id;
        let guild_id = msg.guild_id.clone();

        let mut opts = GamesListOptions::new();
        match args.single::<u32>() {
            Ok(id) => opts.id(Operator::Equals, id),
            Err(_) => opts.fulltext(args.rest().to_string()),
        };
        let task = self
            .modio
            .games()
            .list(&opts)
            .and_then(|mut list| Ok(list.shift()))
            .and_then(move |game| {
                if let Some(g) = game {
                    let ret = Subscriptions::add(&mut ctx2, g.id, channel_id, guild_id);
                    match ret {
                        Ok(_) => {
                            let _ = channel_id.say(format!("Subscribed to '{}'", g.name));
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
                Ok(())
            })
            .map_err(|e| {
                eprintln!("{}", e);
            });

        self.executor.spawn(task);
    }

    options(opts) {
        opts.min_args = Some(1);
    }
);

command!(
    Unsubscribe(self, ctx, msg, args) {
        let mut ctx2 = ctx.clone();
        let channel_id = msg.channel_id;
        let guild_id = msg.guild_id.clone();

        let mut opts = GamesListOptions::new();
        match args.single::<u32>() {
            Ok(id) => opts.id(Operator::Equals, id),
            Err(_) => opts.fulltext(args.rest().to_string()),
        };
        let task = self
            .modio
            .games()
            .list(&opts)
            .and_then(|mut list| Ok(list.shift()))
            .and_then(move |game| {
                if let Some(g) = game {
                    let ret = Subscriptions::remove(&mut ctx2, g.id, channel_id, guild_id);
                    match ret {
                        Ok(_) => {
                            let _ = channel_id.say(format!("Unsubscribed to '{}'", g.name));
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
                Ok(())
            })
            .map_err(|e| {
                eprintln!("{}", e);
            });

        self.executor.spawn(task);
    }
);

pub fn task(
    client: &Client,
    modio: Modio,
    exec: TaskExecutor,
) -> impl Future<Item = (), Error = ()> {
    let data = client.data.clone();

    Interval::new_interval(Duration::from_secs(3 * 60))
        .for_each(move |_| {
            let tstamp = util::current_timestamp() - 3 * 30;
            let mut opts = EventListOptions::new();
            opts.date_added(Operator::GreaterThan, tstamp);
            opts.sort_by(EventListOptions::ID, Order::Asc);

            let data = data.lock();
            let Subscriptions(subs) = data
                .get::<Subscriptions>()
                .expect("failed to get subscriptions");

            for (game, channels) in subs.clone() {
                if channels.is_empty() {
                    continue;
                }
                debug!("polling events for game={} channels: {:?}", game, channels);
                let task = modio
                    .game(game)
                    .mods()
                    .events(&opts)
                    .collect()
                    .and_then(move |events| {
                        for e in events {
                            for (channel, _) in &channels {
                                let _ = channel.say(format!(
                                    "[{}] {:?}",
                                    tstamp,
                                    e,
                                ));
                            }
                        }
                        Ok(())
                    })
                    .map_err(|_| ());

                exec.spawn(task);
            }

            Ok(())
        })
        .map_err(|e| warn!("interval errored: {}", e))
}
