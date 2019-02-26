use std::time::Duration;

use futures::future::{self, Either};
use futures::{Future, Stream};
use log::{debug, warn};
use modio::filter::{Operator, Order};
use modio::games::GamesListOptions;
use modio::mods::ModsListOptions;
use modio::EventListOptions;
use modio::Modio;
use serenity::prelude::*;
use tokio::runtime::TaskExecutor;
use tokio::timer::Interval;

use crate::db::Subscriptions;
use crate::util;

const INTERVAL_DURATION: Duration = Duration::from_secs(300);

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

    Interval::new_interval(INTERVAL_DURATION)
        .fold(util::current_timestamp(), move |tstamp, _| {
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
                debug!(
                    "polling events at {} for game={} channels: {:?}",
                    tstamp, game, channels
                );
                let mods = modio.game(game).mods();
                let task = mods
                    .events(&opts)
                    .collect()
                    .and_then(move |events| {
                        if events.is_empty() {
                            return Either::A(future::ok(()));
                        }
                        let mut opts = ModsListOptions::new();
                        opts.id(
                            Operator::In,
                            events.iter().map(|e| e.mod_id).collect::<Vec<_>>(),
                        );
                        Either::B(mods.iter(&opts).collect().and_then(move |mut mods| {
                            mods.sort_by(|a, b| {
                                events
                                    .iter()
                                    .position(|e| e.mod_id == a.id)
                                    .cmp(&events.iter().position(|e| e.mod_id == b.id))
                            });
                            for (e, m) in events.iter().zip(mods.iter()) {
                                for (channel, _) in &channels {
                                    let _ = channel.say(format!("{} {}", m.name, e.event_type));
                                }
                            }
                            Ok(())
                        }))
                    })
                    .map_err(|_| ());

                exec.spawn(task);
            }

            // current timestamp for the next run
            Ok(util::current_timestamp())
        })
        .map(|_| ())
        .map_err(|e| warn!("interval errored: {}", e))
}
