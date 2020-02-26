use std::sync::mpsc;

use futures::future;
use futures::TryFutureExt;
use futures::TryStreamExt;
use modio::filter::prelude::*;
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
    if let Some(g) = game {
        let data = ctx.data.read();
        let subs = data.get::<Subscriptions>().expect("get subs failed");
        let ret = subs.add(g.id, channel_id, guild_id, evts);
        match ret {
            Ok(_) => {
                let _ = channel_id.say(&ctx, format!("Subscribed to '{}'", g.name));
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
