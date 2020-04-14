use std::sync::mpsc;

use futures::future;
use futures::TryFutureExt;
use futures::TryStreamExt;
use modio::filter::prelude::*;
use modio::games::ApiAccessOptions;
use serenity::prelude::*;

use crate::commands::prelude::*;
use crate::db::{Events, Subscriptions};
use crate::util;

#[command]
#[description = "List subscriptions of the current channel to mod updates of a game"]
#[aliases("subs")]
#[required_permissions("MANAGE_CHANNELS")]
pub fn subscriptions(ctx: &mut Context, msg: &Message) -> CommandResult {
    let channel_id = msg.channel_id;
    let data = ctx.data.read();
    let subs = data.get::<Subscriptions>().expect("get subs failed");
    let games = subs.list_games(msg.channel_id)?;

    if !games.is_empty() {
        let modio = data.get::<ModioKey>().expect("get modio failed");
        let exec = data.get::<ExecutorKey>().expect("get exec failed");
        let (tx, rx) = mpsc::channel();

        let filter = Id::_in(games.keys().collect::<Vec<_>>());
        let task = modio.games().search(filter).iter().try_fold(
            util::ContentBuilder::default(),
            move |mut buf, g| {
                let evts = games.get(&g.id).unwrap_or(&Events::ALL);
                let suffix = match (evts.contains(Events::NEW), evts.contains(Events::UPD)) {
                    (true, true) | (false, false) => " (+Δ)",
                    (true, false) => " (+)",
                    (false, true) => " (Δ)",
                };
                let _ = writeln!(&mut buf, "{}. {} {}", g.id, g.name, suffix);
                future::ok(buf)
            },
        );

        exec.spawn(async move {
            match task.await {
                Ok(games) => tx.send(games).unwrap(),
                Err(e) => eprintln!("{}", e),
            }
        });

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
#[sub_commands(subscribe_new, subscribe_update)]
#[min_args(1)]
#[required_permissions("MANAGE_CHANNELS")]
pub fn subscribe(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    _subscribe(ctx, msg, args, Events::all())
}

#[command]
#[description = "Subscribe the current channel to new mod notifications"]
#[aliases("new")]
#[min_args(1)]
#[required_permissions("MANAGE_CHANNELS")]
pub fn subscribe_new(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    _subscribe(ctx, msg, args, Events::NEW)
}

#[command]
#[description = "Subscribe the current channel to mod update notifications"]
#[aliases("upd")]
#[min_args(1)]
#[required_permissions("MANAGE_CHANNELS")]
pub fn subscribe_update(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    _subscribe(ctx, msg, args, Events::UPD)
}

#[command]
#[description = "Unsubscribe the current channel from mod updates of a game"]
#[aliases("unsub")]
#[sub_commands(unsubscribe_new, unsubscribe_update)]
#[min_args(1)]
#[required_permissions("MANAGE_CHANNELS")]
pub fn unsubscribe(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    _unsubscribe(ctx, msg, args, Events::all())
}

#[command]
#[description = "Unsubscribe the current channel from new mod notifications"]
#[aliases("new")]
#[min_args(1)]
#[required_permissions("MANAGE_CHANNELS")]
pub fn unsubscribe_new(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    _unsubscribe(ctx, msg, args, Events::NEW)
}

#[command]
#[description = "Unsubscribe the current channel from mod update notification"]
#[aliases("upd")]
#[min_args(1)]
#[required_permissions("MANAGE_CHANNELS")]
pub fn unsubscribe_update(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    _unsubscribe(ctx, msg, args, Events::UPD)
}

#[command]
#[description = "List muted mods"]
#[required_permissions("MANAGE_CHANNELS")]
pub fn muted(ctx: &mut Context, msg: &Message) -> CommandResult {
    let channel_id = msg.channel_id;
    let data = ctx.data.read();
    let subs = data.get::<Subscriptions>().expect("get subs failed");
    let modio = data.get::<ModioKey>().expect("get modio failed");
    let exec = data.get::<ExecutorKey>().expect("get exec failed");
    let (tx, rx) = mpsc::channel();

    let excluded = subs.list_excluded(msg.channel_id)?;

    match excluded.len() {
        0 => {
            msg.channel_id.say(&ctx, "No mod is muted")?;
        }
        1 => {
            let (game, mods) = excluded.into_iter().next().unwrap();
            let filter = Id::_in(mods.into_iter().collect::<Vec<_>>());
            let task = modio.game(game).mods().search(filter).iter().try_fold(
                util::ContentBuilder::default(),
                |mut buf, m| {
                    let _ = writeln!(&mut buf, "{}. {}", m.id, m.name);
                    future::ok(buf)
                },
            );

            exec.spawn(async move {
                match task.await {
                    Ok(games) => tx.send(games).unwrap(),
                    Err(e) => eprintln!("{}", e),
                }
            });
        }
        _ => {
            let task = excluded
                .into_iter()
                .map(|(game, mods)| {
                    let filter = Id::_in(mods.into_iter().collect::<Vec<_>>());
                    future::try_join(
                        modio.game(game).get(),
                        modio.game(game).mods().search(filter).collect(),
                    )
                })
                .collect::<futures::stream::FuturesUnordered<_>>()
                .try_fold(util::ContentBuilder::default(), |mut buf, (game, mods)| {
                    let _ = writeln!(&mut buf, "**{}**", game.name);
                    for m in mods {
                        let _ = writeln!(&mut buf, "{}. {}", m.id, m.name);
                    }
                    let _ = writeln!(&mut buf, "");
                    future::ok(buf)
                });

            exec.spawn(async move {
                match task.await {
                    Ok(games) => tx.send(games).unwrap(),
                    Err(e) => eprintln!("{}", e),
                }
            });
        }
    }

    let muted = rx.recv().unwrap();
    for content in muted {
        let _ = channel_id.send_message(&ctx, |m| {
            m.embed(|e| e.title("Muted mods").description(content))
        });
    }
    Ok(())
}

#[command]
#[description = "Mute update notifications for a mod"]
#[min_args(2)]
#[required_permissions("MANAGE_CHANNELS")]
pub fn mute(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let game_filter = match args.single::<u32>() {
        Ok(id) => Id::eq(id),
        Err(_) => Fulltext::eq(args.quoted().single::<String>()?),
    };
    let mod_filter = match args.single::<u32>() {
        Ok(id) => Id::eq(id),
        Err(_) => Fulltext::eq(args.quoted().single::<String>()?),
    };

    let data = ctx.data.read();
    let modio = data.get::<ModioKey>().expect("get modio failed");
    let subs = data.get::<Subscriptions>().expect("get subs failed");
    let exec = data.get::<ExecutorKey>().expect("get exec failed");
    let (tx, rx) = mpsc::channel();

    let task = find_game_mod(modio.clone(), game_filter, mod_filter);

    exec.spawn(async move {
        match task.await {
            Ok(game) => tx.send(game).unwrap(),
            Err(e) => eprintln!("{}", e),
        }
    });

    let channel = msg.channel_id;
    match rx.recv().unwrap() {
        (None, _) => {
            channel.say(&ctx, "Game not found")?;
        }
        (_, None) => {
            channel.say(&ctx, "Mod not found")?;
        }
        (Some(game), Some(mod_)) => {
            if let Err(e) = subs.mute_mod(game.id, msg.channel_id, msg.guild_id, mod_.id) {
                eprintln!("{}", e);
                channel.say(&ctx, format!("Failed to mute '{}'", mod_.name))?;
            } else {
                channel.say(&ctx, format!("The mod '{}' is now muted", mod_.name))?;
            }
        }
    }
    Ok(())
}

#[command]
#[description = "Unmute update notifications for a mod"]
#[min_args(2)]
#[required_permissions("MANAGE_CHANNELS")]
pub fn unmute(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let game_filter = match args.single::<u32>() {
        Ok(id) => Id::eq(id),
        Err(_) => Fulltext::eq(args.quoted().single::<String>()?),
    };
    let mod_filter = match args.single::<u32>() {
        Ok(id) => Id::eq(id),
        Err(_) => Fulltext::eq(args.quoted().single::<String>()?),
    };

    let data = ctx.data.read();
    let modio = data.get::<ModioKey>().expect("get modio failed");
    let subs = data.get::<Subscriptions>().expect("get subs failed");
    let exec = data.get::<ExecutorKey>().expect("get exec failed");
    let (tx, rx) = mpsc::channel();

    let task = find_game_mod(modio.clone(), game_filter, mod_filter);

    exec.spawn(async move {
        match task.await {
            Ok(game) => tx.send(game).unwrap(),
            Err(e) => eprintln!("{}", e),
        }
    });

    let channel = msg.channel_id;
    match rx.recv().unwrap() {
        (None, _) => {
            channel.say(&ctx, "Game not found")?;
        }
        (_, None) => {
            channel.say(&ctx, "Mod not found")?;
        }
        (Some(game), Some(mod_)) => {
            if let Err(e) = subs.unmute_mod(game.id, msg.channel_id, mod_.id) {
                eprintln!("{}", e);
                channel.say(&ctx, format!("Failed to unmute '{}'", mod_.name))?;
            } else {
                channel.say(&ctx, format!("The mod '{}' is now unmuted", mod_.name))?;
            }
        }
    }
    Ok(())
}

fn _subscribe(ctx: &mut Context, msg: &Message, mut args: Args, evts: Events) -> CommandResult {
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

        let task = modio.games().search(filter).first().and_then(|mut list| {
            let game = if list.is_empty() {
                None
            } else {
                Some(list.remove(0))
            };
            future::ok(game)
        });

        exec.spawn(async move {
            match task.await {
                Ok(game) => tx.send(game).unwrap(),
                Err(e) => eprintln!("{}", e),
            }
        });
        rx.recv().unwrap()
    };
    if let Some(game) = game {
        if !game
            .api_access_options
            .contains(ApiAccessOptions::ALLOW_THIRD_PARTY)
        {
            let msg = format!(
                ":no_entry: Third party API access is disabled for '{}' but is required for subscriptions.",
                game.name
            );
            let _ = channel_id.say(&ctx, msg);
            return Ok(());
        }
        let data = ctx.data.read();
        let subs = data.get::<Subscriptions>().expect("get subs failed");
        let ret = subs.add(game.id, channel_id, guild_id, evts);
        match ret {
            Ok(_) => {
                let _ = channel_id.say(&ctx, format!("Subscribed to '{}'", game.name));
            }
            Err(e) => eprintln!("{}", e),
        }
    }
    Ok(())
}

fn _unsubscribe(ctx: &mut Context, msg: &Message, mut args: Args, evts: Events) -> CommandResult {
    let channel_id = msg.channel_id;

    let game = {
        let data = ctx.data.read();
        let modio = data.get::<ModioKey>().expect("get modio failed");
        let exec = data.get::<ExecutorKey>().expect("get exec failed");
        let (tx, rx) = mpsc::channel();

        let filter = match args.single::<u32>() {
            Ok(id) => Id::eq(id),
            Err(_) => Fulltext::eq(args.rest().to_string()),
        };
        let task = modio.games().search(filter).first().and_then(|mut list| {
            let game = if list.is_empty() {
                None
            } else {
                Some(list.remove(0))
            };
            future::ok(game)
        });

        exec.spawn(async move {
            match task.await {
                Ok(game) => tx.send(game).unwrap(),
                Err(e) => eprintln!("{}", e),
            }
        });

        rx.recv().unwrap()
    };

    if let Some(g) = game {
        let data = ctx.data.read();
        let subs = data.get::<Subscriptions>().expect("get subs failed");
        let ret = subs.remove(g.id, channel_id, evts);
        match ret {
            Ok(_) => {
                let _ = channel_id.say(&ctx, format!("Unsubscribed to '{}'", g.name));
            }
            Err(e) => eprintln!("{}", e),
        }
    }
    Ok(())
}

use modio::filter::Filter;
use modio::games::Game;
use modio::mods::Mod;
use modio::{Modio, Result};

async fn find_game_mod(
    modio: Modio,
    game_filter: Filter,
    mod_filter: Filter,
) -> Result<(Option<Game>, Option<Mod>)> {
    let mut games = modio.games().search(game_filter).first().await?;
    if games.is_empty() {
        return Ok((None, None));
    }
    let game = games.remove(0);

    let mut mods = modio
        .game(game.id)
        .mods()
        .search(mod_filter)
        .first()
        .await?;

    if mods.is_empty() {
        return Ok((Some(game), None));
    }
    Ok((Some(game), Some(mods.remove(0))))
}
