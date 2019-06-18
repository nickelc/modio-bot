use std::sync::mpsc;
use std::time::Duration;

use futures::future::{self, Either};
use futures::{Future, Stream};
use log::{debug, warn};
use modio::filter::prelude::*;
use modio::games::Game;
use modio::mods::filters::events::EventType as EventTypeFilter;
use modio::mods::{Event, EventType, Mod};
use modio::users::filters::Id as UserId;
use modio::users::User;
use modio::Modio;
use serenity::prelude::*;
use tokio::runtime::TaskExecutor;
use tokio::timer::Interval;

use crate::commands::prelude::*;
use crate::db::Subscriptions;
use crate::util;

const INTERVAL_DURATION: Duration = Duration::from_secs(300);

#[command]
#[description = "List subscriptions of the current channel to mod updates of a game"]
#[aliases("subs")]
#[required_permissions("MANAGE_CHANNELS")]
pub fn subscriptions(ctx: &mut Context, msg: &Message) -> CommandResult {
    let mut ctx2 = ctx.clone();
    let channel_id = msg.channel_id;
    let games = Subscriptions::list_games(&mut ctx2, msg.channel_id);

    if !games.is_empty() {
        let data = ctx.data.read();
        let modio = data.get::<ModioKey>().expect("get modio failed");
        let exec = data.get::<ExecutorKey>().expect("get exec failed");
        let (tx, rx) = mpsc::channel();

        let filter = Id::_in(games);
        let task = modio
            .games()
            .iter(&filter)
            .fold(util::ContentBuilder::default(), |mut buf, g| {
                let _ = writeln!(&mut buf, "{}. {}", g.id, g.name);
                future::ok::<_, modio::error::Error>(buf)
            })
            .and_then(move |games| {
                tx.send(games).unwrap();
                Ok(())
            })
            .map_err(|e| eprintln!("{}", e));
        exec.spawn(task);

        let games = rx.recv().unwrap();
        for content in games {
            let _ = channel_id.send_message(&ctx, |m| {
                m.embed(|e| e.title("Subscriptions").description(content))
            });
        }
    } else {
        let _ = channel_id.say(&ctx, "No subscriptions found.");
    }
    Ok(())
}

#[command]
#[description = "Subscribe the current channel to mod updates of a game"]
#[aliases("sub")]
#[min_args(1)]
#[required_permissions("MANAGE_CHANNELS")]
pub fn subscribe(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let channel_id = msg.channel_id;
    let guild_id = msg.guild_id;

    let filter = match args.single::<u32>() {
        Ok(id) => Id::eq(id),
        Err(_) => Fulltext::eq(args.rest().to_string()),
    };

    let game = {
        let data = ctx.data.read();
        let modio = data.get::<ModioKey>().expect("get modio failed");
        let exec = data.get::<ExecutorKey>().expect("get exec failed");
        let (tx, rx) = mpsc::channel();

        let task = modio
            .games()
            .list(&filter)
            .and_then(|mut list| Ok(list.shift()))
            .and_then(move |game| {
                tx.send(game).unwrap();
                Ok(())
            })
            .map_err(|e| {
                eprintln!("{}", e);
            });

        exec.spawn(task);
        rx.recv().unwrap()
    };
    if let Some(g) = game {
        let mut ctx2 = ctx.clone();
        let ret = Subscriptions::add(&mut ctx2, g.id, channel_id, guild_id);
        match ret {
            Ok(_) => {
                let _ = channel_id.say(&ctx, format!("Subscribed to '{}'", g.name));
            }
            Err(e) => eprintln!("{}", e),
        }
    }
    Ok(())
}

#[command]
#[description = "Unsubscribe the current channel from mod updates of a game"]
#[aliases("unsub")]
#[min_args(1)]
#[required_permissions("MANAGE_CHANNELS")]
pub fn unsubscribe(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let channel_id = msg.channel_id;
    let guild_id = msg.guild_id;

    let game = {
        let data = ctx.data.read();
        let modio = data.get::<ModioKey>().expect("get modio failed");
        let exec = data.get::<ExecutorKey>().expect("get exec failed");
        let (tx, rx) = mpsc::channel();

        let filter = match args.single::<u32>() {
            Ok(id) => Id::eq(id),
            Err(_) => Fulltext::eq(args.rest().to_string()),
        };
        let task = modio
            .games()
            .list(&filter)
            .and_then(|mut list| Ok(list.shift()))
            .and_then(move |game| {
                tx.send(game).unwrap();
                Ok(())
            })
            .map_err(|e| {
                eprintln!("{}", e);
            });

        exec.spawn(task);

        rx.recv().unwrap()
    };

    if let Some(g) = game {
        let mut ctx2 = ctx.clone();
        let ret = Subscriptions::remove(&mut ctx2, g.id, channel_id, guild_id);
        match ret {
            Ok(_) => {
                let _ = channel_id.say(&ctx, format!("Unsubscribed to '{}'", g.name));
            }
            Err(e) => eprintln!("{}", e),
        }
    }
    Ok(())
}

/*
struct Notification<'a> {
    event: &'a Event,
    user: &'a User,
    mod_: &'a Mod,
}

impl<'a> Notification<'a> {
    fn new((event, (user, mod_)): (&'a Event, (&'a User, &'a Mod))) -> Notification<'a> {
        Notification { event, user, mod_ }
    }

    fn is_ignored(&self) -> bool {
        use EventType::*;
        match self.event.event_type {
            UserTeamJoin | UserTeamLeave | UserSubscribe | UserUnsubscribe | ModTeamChanged => true,
            _ => false,
        }
    }

    fn create_message(&self, game: &Game, m: CreateMessage) -> CreateMessage {
        use crate::commands::mods::ModExt;

        let create_embed =
            |m: CreateMessage, desc: &str, changelog: Option<(&str, String, bool)>| {
                m.embed(|e| {
                    e.title(&self.mod_.name)
                        .url(&self.mod_.profile_url)
                        .description(desc)
                        .thumbnail(&self.mod_.logo.thumb_320x180)
                        .author(|a| {
                            a.name(&game.name)
                                .icon_url(&game.icon.thumb_64x64.to_string())
                                .url(&game.profile_url.to_string())
                        })
                        .footer(|f| self.user.create_footer(f))
                        .fields(changelog)
                })
            };

        match self.event.event_type {
            EventType::ModEdited => create_embed(m, "The mod has been edited.", None),
            EventType::ModAvailable => {
                let m = m.content("A new mod is available. :tada:");
                self.mod_.create_new_mod_message(game, m)
            }
            EventType::ModUnavailable => create_embed(m, "The mod is now unavailable.", None),
            EventType::ModfileChanged => {
                let (desc, changelog) = self
                    .mod_
                    .modfile
                    .as_ref()
                    .map(|f| {
                        let link = &f.download.binary_url;
                        let no_version = || format!("[Download]({})", link);
                        let version = |v| format!("[Version {}]({})", v, link);
                        let download = f
                            .version
                            .as_ref()
                            .filter(|v| !v.is_empty())
                            .map_or_else(no_version, version);
                        let changelog = f
                            .changelog
                            .as_ref()
                            .filter(|c| !c.is_empty())
                            .map(|c| {
                                let it = c.char_indices().rev().scan(c.len(), |state, (pos, _)| {
                                    if *state > 1024 {
                                        *state = pos;
                                        Some(pos)
                                    } else {
                                        None
                                    }
                                });
                                let pos = it.last().unwrap_or(c.len());
                                &c[..pos]
                            })
                            .map(|c| ("Changelog", c.to_owned(), true));
                        let desc = format!("A new version is available. {}", download);

                        (desc, changelog)
                    })
                    .unwrap_or_default();
                create_embed(m, &desc, changelog)
            }
            EventType::ModDeleted => create_embed(m, "The mod has been permanently deleted.", None),
            _ => create_embed(m, "event ignored", None),
        }
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
                    // EventType::ModEdited,
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

                let users = modio.users();
                let game = modio.game(game);
                let mods = game.mods();
                let task = mods
                    .events(&filter)
                    .collect()
                    .and_then(move |events| {
                        if events.is_empty() {
                            return Either::A(future::ok(()));
                        }
                        let (mid, uid): (Vec<_>, Vec<_>) =
                            events.iter().map(|e| (e.mod_id, e.user_id)).unzip();
                        let filter = Id::_in(mid);

                        let game = game.get();
                        let mods = mods.iter(&filter).collect();
                        let users = users.iter(&UserId::_in(uid)).collect();

                        Either::B(game.join(mods).join(users).and_then(
                            move |((game, mods), users)| {
                                let mods = events
                                    .iter()
                                    .map(|e| mods.iter().find(|m| m.id == e.mod_id))
                                    .flatten();
                                let users = events
                                    .iter()
                                    .map(|e| users.iter().find(|u| u.id == e.user_id))
                                    .flatten();
                                let it = events
                                    .iter()
                                    .zip(users.zip(mods))
                                    .map(Notification::new)
                                    .filter(|n| !n.is_ignored());
                                for n in it {
                                    for (channel, _) in &channels {
                                        debug!(
                                            "send message to #{}: {} for {:?}",
                                            channel, n.event.event_type, n.mod_.name,
                                        );
                                        let _ =
                                            channel.send_message(|m| n.create_message(&game, m));
                                    }
                                }
                                Ok(())
                            },
                        ))
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
*/
