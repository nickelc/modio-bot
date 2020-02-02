use std::sync::mpsc;

use futures::future;
use futures::TryFutureExt;
use futures::TryStreamExt;
use modio::filter::prelude::*;
use serenity::prelude::*;

use crate::commands::prelude::*;
use crate::db::Subscriptions;
use crate::util;

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
        let task =
            modio
                .games()
                .iter(filter)
                .try_fold(util::ContentBuilder::default(), |mut buf, g| {
                    let _ = writeln!(&mut buf, "{}. {}", g.id, g.name);
                    future::ok(buf)
                });

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
            .list(filter)
            .and_then(|mut list| future::ok(list.shift()));

        exec.spawn(async move {
            match task.await {
                Ok(game) => tx.send(game).unwrap(),
                Err(e) => eprintln!("{}", e),
            }
        });
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
            .list(filter)
            .and_then(|mut list| future::ok(list.shift()));

        exec.spawn(async move {
            match task.await {
                Ok(game) => tx.send(game).unwrap(),
                Err(e) => eprintln!("{}", e),
            }
        });

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
