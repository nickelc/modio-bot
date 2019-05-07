use std::time::Duration;

use either::{Left, Right};
use futures::future::{self, Either};
use futures::{Future, Stream};
use log::{debug, warn};
use modio::filter::prelude::*;
use modio::mods::filters::events::EventType as EventTypeFilter;
use modio::mods::{Event, EventType, Mod};
use modio::Modio;
use serenity::model::permissions::Permissions;
use serenity::prelude::*;
use tokio::runtime::TaskExecutor;
use tokio::timer::Interval;

use crate::commands::prelude::*;
use crate::db::Subscriptions;
use crate::util;

const INTERVAL_DURATION: Duration = Duration::from_secs(300);

command!(
    List(self, ctx, msg) {
        let mut ctx2 = ctx;
        let channel_id = msg.channel_id;
        let games = Subscriptions::list_games(&mut ctx2, msg.channel_id);

        if !games.is_empty() {
            let filter = Id::_in(games);
            let task = self
                .modio
                .games()
                .iter(&filter)
                .fold(util::ContentBuilder::default(), |mut buf, g| {
                    let _ = writeln!(&mut buf, "{}. {}", g.id, g.name);
                    future::ok::<_, modio::error::Error>(buf)
                })
                .and_then(move |games| {
                    for content in games {
                        let _ = channel_id.send_message(|m| {
                            m.embed(|e| e.title("Subscriptions").description(content))
                        });
                    }
                    Ok(())
                })
                .map_err(|e| eprintln!("{}", e));
            self.executor.spawn(task);
        } else {
            let _ = channel_id.say("No subscriptions found.");
        }
    }

    options(opts) {
        opts.desc = Some("List subscriptions of the current channel to mod updates of a game".to_string());
        opts.aliases = vec!["subs".to_string()];
        opts.required_permissions = Permissions::MANAGE_CHANNELS;
    }
);

command!(
    Subscribe(self, ctx, msg, args) {
        let mut ctx2 = ctx.clone();
        let channel_id = msg.channel_id;
        let guild_id = msg.guild_id;

        let filter = match args.single::<u32>() {
            Ok(id) => Id::eq(id),
            Err(_) => Fulltext::eq(args.rest().to_string()),
        };
        let task = self
            .modio
            .games()
            .list(&filter)
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
        opts.desc = Some("Subscribe the current channel to mod updates of a game".to_string());
        opts.min_args = Some(1);
        opts.required_permissions = Permissions::MANAGE_CHANNELS;
    }
);

command!(
    Unsubscribe(self, ctx, msg, args) {
        let mut ctx2 = ctx.clone();
        let channel_id = msg.channel_id;
        let guild_id = msg.guild_id;

        let filter = match args.single::<u32>() {
            Ok(id) => Id::eq(id),
            Err(_) => Fulltext::eq(args.rest().to_string()),
        };
        let task = self
            .modio
            .games()
            .list(&filter)
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

    options(opts) {
        opts.desc = Some("Unsubscribe the current channel from mod updates of a game".to_string());
        opts.min_args = Some(1);
        opts.required_permissions = Permissions::MANAGE_CHANNELS;
    }
);

struct Notification<'a> {
    event: &'a Event,
    mod_: &'a Mod,
}

impl<'a> Notification<'a> {
    fn new((event, mod_): (&'a Event, &'a Mod)) -> Notification<'a> {
        Notification { event, mod_ }
    }

    fn is_ignored(&self) -> bool {
        use EventType::*;
        match self.event.event_type {
            UserTeamJoin | UserTeamLeave | UserSubscribe | UserUnsubscribe | ModTeamChanged => true,
            _ => false,
        }
    }

    fn create_message(&self, m: CreateMessage) -> CreateMessage {
        let desc = match self.event.event_type {
            EventType::ModEdited => Left("The mod has been edited."),
            EventType::ModAvailable => Left("A new mod is available."),
            EventType::ModUnavailable => Left("The mod is now unavailable."),
            EventType::ModfileChanged => Right(format!(
                "The primary file changed. [link]({})",
                self.mod_
                    .modfile
                    .as_ref()
                    .map(|f| f.download.binary_url.to_string())
                    .unwrap_or_default(),
            )),
            EventType::ModDeleted => Left("The mod has been permanently deleted."),
            _ => Left("ignored event"),
        };
        m.embed(|e| {
            e.title(&self.mod_.name)
                .url(&self.mod_.profile_url)
                .description(desc)
                .thumbnail(&self.mod_.logo.thumb_320x180)
        })
    }
}

pub fn task(
    client: &Client,
    modio: Modio,
    exec: TaskExecutor,
) -> impl Future<Item = (), Error = ()> {
    let data = client.data.clone();

    Interval::new_interval(INTERVAL_DURATION)
        .fold(util::current_timestamp(), move |tstamp, _| {
            let filter = DateAdded::gt(tstamp)
                .and(EventTypeFilter::_in(vec![
                    EventType::ModfileChanged,
                    EventType::ModEdited,
                    EventType::ModDeleted,
                    EventType::ModAvailable,
                    EventType::ModUnavailable,
                ]))
                .order_by(Id::asc());

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
                    .events(&filter)
                    .collect()
                    .and_then(move |events| {
                        if events.is_empty() {
                            return Either::A(future::ok(()));
                        }
                        let filter = Id::_in(events.iter().map(|e| e.mod_id).collect::<Vec<_>>());

                        Either::B(mods.iter(&filter).collect().and_then(move |mut mods| {
                            mods.sort_by(|a, b| {
                                events
                                    .iter()
                                    .position(|e| e.mod_id == a.id)
                                    .cmp(&events.iter().position(|e| e.mod_id == b.id))
                            });
                            let it = events
                                .iter()
                                .zip(mods.iter())
                                .map(Notification::new)
                                .filter(|n| !n.is_ignored());
                            for n in it {
                                for (channel, _) in &channels {
                                    let _ = channel.send_message(|m| n.create_message(m));
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