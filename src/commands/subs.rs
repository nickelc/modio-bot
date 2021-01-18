use std::collections::HashMap;

use futures_util::stream::FuturesUnordered;
use modio::filter::prelude::*;
use modio::games::ApiAccessOptions;
use serenity::prelude::*;

use crate::commands::prelude::*;
use crate::db::{Events, Subscriptions, Tags};
use crate::util;

#[command]
#[description = "List subscriptions of the current channel to mod updates of a game"]
#[aliases("subs")]
#[required_permissions("MANAGE_CHANNELS")]
pub async fn subscriptions(ctx: &Context, msg: &Message) -> CommandResult {
    let channel_id = msg.channel_id;
    let data = ctx.data.read().await;
    let subs = data.get::<Subscriptions>().expect("get subs failed");
    let subs = subs.list_for_channel(msg.channel_id)?;

    if !subs.is_empty() {
        let modio = data.get::<ModioKey>().expect("get modio failed");

        let filter = Id::_in(subs.iter().map(|s| s.0).collect::<Vec<_>>());
        let list = modio.games().search(filter).collect().await?;
        let games = list
            .into_iter()
            .map(|g| (g.id, g.name))
            .collect::<HashMap<_, _>>();
        let mut buf = util::ContentBuilder::default();
        for (game_id, tags, evts) in subs {
            let suffix = match (evts.contains(Events::NEW), evts.contains(Events::UPD)) {
                (true, true) | (false, false) => " (+Δ)",
                (true, false) => " (+)",
                (false, true) => " (Δ)",
            };
            let tags = if tags.is_empty() {
                String::new()
            } else {
                let mut buf = String::from(" | Tags: ");
                push_tags(&mut buf, tags.iter());
                buf
            };
            let name = games.get(&game_id).unwrap();
            let _ = writeln!(&mut buf, "{}. {} {}{}", game_id, name, suffix, tags);
        }

        for content in buf {
            let _ = channel_id
                .send_message(ctx, |m| {
                    m.embed(|e| e.title("Subscriptions").description(content))
                })
                .await;
        }
    } else {
        let _ = channel_id.say(ctx, "No subscriptions found.").await;
    }
    Ok(())
}

#[command]
#[description = "Subscribe the current channel to mod updates of a game"]
#[aliases("sub")]
#[sub_commands(subscribe_new, subscribe_update)]
#[min_args(1)]
#[required_permissions("MANAGE_CHANNELS")]
#[usage("<name or id> [tag ..]")]
#[example("snowrunner")]
#[example("xcom \"UFO Defense\" Major")]
pub async fn subscribe(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    _subscribe(ctx, msg, args, Events::all()).await
}

#[command]
#[description = "Subscribe the current channel to new mod notifications"]
#[aliases("new")]
#[min_args(1)]
#[required_permissions("MANAGE_CHANNELS")]
#[usage("<name or id> [tag ..]")]
#[example("snowrunner")]
#[example("xcom \"UFO Defense\" Major")]
pub async fn subscribe_new(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    _subscribe(ctx, msg, args, Events::NEW).await
}

#[command]
#[description = "Subscribe the current channel to mod update notifications"]
#[aliases("upd")]
#[min_args(1)]
#[required_permissions("MANAGE_CHANNELS")]
#[usage("<name or id> [tag ..]")]
#[example("snowrunner")]
#[example("xcom \"UFO Defense\" Major")]
pub async fn subscribe_update(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    _subscribe(ctx, msg, args, Events::UPD).await
}

#[command]
#[description = "Unsubscribe the current channel from mod updates of a game"]
#[aliases("unsub")]
#[sub_commands(unsubscribe_new, unsubscribe_update)]
#[min_args(1)]
#[required_permissions("MANAGE_CHANNELS")]
#[usage("<name or id> [tag ..]")]
#[example("snowrunner")]
#[example("xcom \"UFO Defense\" Major")]
pub async fn unsubscribe(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    _unsubscribe(ctx, msg, args, Events::all()).await
}

#[command]
#[description = "Unsubscribe the current channel from new mod notifications"]
#[aliases("new")]
#[min_args(1)]
#[required_permissions("MANAGE_CHANNELS")]
#[usage("<name or id> [tag ..]")]
#[example("snowrunner")]
#[example("xcom \"UFO Defense\" Major")]
pub async fn unsubscribe_new(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    _unsubscribe(ctx, msg, args, Events::NEW).await
}

#[command]
#[description = "Unsubscribe the current channel from mod update notification"]
#[aliases("upd")]
#[min_args(1)]
#[required_permissions("MANAGE_CHANNELS")]
#[usage("<name or id> [tag ..]")]
#[example("snowrunner")]
#[example("xcom \"UFO Defense\" Major")]
pub async fn unsubscribe_update(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    _unsubscribe(ctx, msg, args, Events::UPD).await
}

#[command]
#[description = "List muted mods"]
#[required_permissions("MANAGE_CHANNELS")]
pub async fn muted(ctx: &Context, msg: &Message) -> CommandResult {
    let channel_id = msg.channel_id;
    let data = ctx.data.read().await;
    let subs = data.get::<Subscriptions>().expect("get subs failed");
    let modio = data.get::<ModioKey>().expect("get modio failed");

    let excluded = subs.list_excluded_mods(msg.channel_id)?;

    let muted = match excluded.len() {
        0 => {
            msg.channel_id.say(ctx, "No mod is muted").await?;
            return Ok(());
        }
        1 => {
            let (game, mods) = excluded.into_iter().next().unwrap();
            let filter = Id::_in(mods.into_iter().collect::<Vec<_>>());
            modio
                .game(game)
                .mods()
                .search(filter)
                .iter()
                .await?
                .try_fold(util::ContentBuilder::default(), |mut buf, m| {
                    let _ = writeln!(&mut buf, "{}. {}", m.id, m.name);
                    future::ok(buf)
                })
                .await?
        }
        _ => {
            excluded
                .into_iter()
                .map(|(game, mods)| {
                    let filter = Id::_in(mods.into_iter().collect::<Vec<_>>());
                    future::try_join(
                        modio.game(game).get(),
                        modio.game(game).mods().search(filter).collect(),
                    )
                })
                .collect::<FuturesUnordered<_>>()
                .try_fold(util::ContentBuilder::default(), |mut buf, (game, mods)| {
                    let _ = writeln!(&mut buf, "**{}**", game.name);
                    for m in mods {
                        let _ = writeln!(&mut buf, "{}. {}", m.id, m.name);
                    }
                    let _ = writeln!(&mut buf);
                    future::ok(buf)
                })
                .await?
        }
    };

    for content in muted {
        let _ = channel_id
            .send_message(ctx, |m| {
                m.embed(|e| e.title("Muted mods").description(content))
            })
            .await;
    }
    Ok(())
}

#[command]
#[description = "Mute update notifications for a mod"]
#[min_args(2)]
#[required_permissions("MANAGE_CHANNELS")]
pub async fn mute(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let game_filter = match args.single::<u32>() {
        Ok(id) => Id::eq(id),
        Err(_) => Fulltext::eq(args.quoted().single::<String>()?),
    };
    let mod_filter = match args.single::<u32>() {
        Ok(id) => Id::eq(id),
        Err(_) => Fulltext::eq(args.quoted().single::<String>()?),
    };

    let data = ctx.data.read().await;
    let modio = data.get::<ModioKey>().expect("get modio failed");
    let subs = data.get::<Subscriptions>().expect("get subs failed");

    let game_mod = find_game_mod(modio.clone(), game_filter, mod_filter).await?;

    let channel = msg.channel_id;

    match game_mod {
        (None, _) => {
            channel.say(ctx, "Game not found").await?;
        }
        (_, None) => {
            channel.say(ctx, "Mod not found").await?;
        }
        (Some(game), Some(mod_)) => {
            if let Err(e) = subs.mute_mod(game.id, msg.channel_id, msg.guild_id, mod_.id) {
                tracing::error!("{}", e);
                channel
                    .say(ctx, format!("Failed to mute '{}'", mod_.name))
                    .await?;
            } else {
                channel
                    .say(ctx, format!("The mod '{}' is now muted", mod_.name))
                    .await?;
            }
        }
    }
    Ok(())
}

#[command]
#[description = "Unmute update notifications for a mod"]
#[min_args(2)]
#[required_permissions("MANAGE_CHANNELS")]
pub async fn unmute(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let game_filter = match args.single::<u32>() {
        Ok(id) => Id::eq(id),
        Err(_) => Fulltext::eq(args.quoted().single::<String>()?),
    };
    let mod_filter = match args.single::<u32>() {
        Ok(id) => Id::eq(id),
        Err(_) => Fulltext::eq(args.quoted().single::<String>()?),
    };

    let data = ctx.data.read().await;
    let modio = data.get::<ModioKey>().expect("get modio failed");
    let subs = data.get::<Subscriptions>().expect("get subs failed");

    let game_mod = find_game_mod(modio.clone(), game_filter, mod_filter).await?;

    let channel = msg.channel_id;

    match game_mod {
        (None, _) => {
            channel.say(ctx, "Game not found").await?;
        }
        (_, None) => {
            channel.say(ctx, "Mod not found").await?;
        }
        (Some(game), Some(mod_)) => {
            if let Err(e) = subs.unmute_mod(game.id, msg.channel_id, mod_.id) {
                tracing::error!("{}", e);
                channel
                    .say(ctx, format!("Failed to unmute '{}'", mod_.name))
                    .await?;
            } else {
                channel
                    .say(ctx, format!("The mod '{}' is now unmuted", mod_.name))
                    .await?;
            }
        }
    }
    Ok(())
}

#[command("muted-users")]
#[description = "List muted users"]
#[required_permissions("MANAGE_CHANNELS")]
pub async fn muted_users(ctx: &Context, msg: &Message) -> CommandResult {
    let channel_id = msg.channel_id;
    let data = ctx.data.read().await;
    let subs = data.get::<Subscriptions>().expect("get subs failed");
    let modio = data.get::<ModioKey>().expect("get modio failed");

    let excluded = subs.list_excluded_users(msg.channel_id)?;

    let muted = match excluded.len() {
        0 => {
            msg.channel_id.say(ctx, "No user is muted").await?;
            return Ok(());
        }
        1 => {
            let (_game, users) = excluded.into_iter().next().unwrap();

            let mut muted = util::ContentBuilder::default();
            for (i, name) in users.iter().enumerate() {
                let _ = writeln!(&mut muted, "{}. {}", i + 1, name);
            }
            muted
        }
        _ => {
            excluded
                .into_iter()
                .map(|(game, users)| {
                    future::try_join(modio.game(game).get(), future::ready(Ok(users)))
                })
                .collect::<FuturesUnordered<_>>()
                .try_fold(util::ContentBuilder::default(), |mut buf, (game, users)| {
                    let _ = writeln!(&mut buf, "**{}**", game.name);
                    for (i, name) in users.iter().enumerate() {
                        let _ = writeln!(&mut buf, "{}. {}", i + 1, name);
                    }
                    let _ = writeln!(&mut buf);
                    future::ok(buf)
                })
                .await?
        }
    };

    for content in muted {
        let _ = channel_id
            .send_message(&ctx, |m| {
                m.embed(|e| e.title("Muted users").description(content))
            })
            .await;
    }
    Ok(())
}

#[command("mute-user")]
#[description = "Mute update notifications for mods of a user"]
#[min_args(2)]
#[usage("<game> <username>")]
#[required_permissions("MANAGE_CHANNELS")]
pub async fn mute_user(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let game_filter = match args.single::<u32>() {
        Ok(id) => Id::eq(id),
        Err(_) => Fulltext::eq(args.quoted().single::<String>()?),
    };
    let name = args.rest();

    let data = ctx.data.read().await;
    let modio = data.get::<ModioKey>().expect("get modio failed");
    let subs = data.get::<Subscriptions>().expect("get subs failed");

    let game = modio.games().search(game_filter).first().await?;

    let channel = msg.channel_id;
    match game {
        None => {
            channel.say(ctx, "Game not found").await?;
        }
        Some(game) => {
            if let Err(e) = subs.mute_user(game.id, msg.channel_id, msg.guild_id, name) {
                tracing::error!("{}", e);
                channel
                    .say(ctx, format!("Failed to mute '{}'", name))
                    .await?;
            } else {
                channel
                    .say(
                        ctx,
                        format!("The user '{}' is now muted for '{}'", name, game.name),
                    )
                    .await?;
            }
        }
    }
    Ok(())
}

#[command("unmute-user")]
#[description = "Unmute update notifications for mods of a user"]
#[min_args(2)]
#[usage("<game> <username>")]
#[required_permissions("MANAGE_CHANNELS")]
pub async fn unmute_user(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let game_filter = match args.single::<u32>() {
        Ok(id) => Id::eq(id),
        Err(_) => Fulltext::eq(args.quoted().single::<String>()?),
    };
    let name = args.rest();

    let data = ctx.data.read().await;
    let modio = data.get::<ModioKey>().expect("get modio failed");
    let subs = data.get::<Subscriptions>().expect("get subs failed");

    let game = modio.games().search(game_filter).first().await?;

    let channel = msg.channel_id;
    match game {
        None => {
            channel.say(ctx, "Game not found").await?;
        }
        Some(game) => {
            if let Err(e) = subs.unmute_user(game.id, msg.channel_id, name) {
                tracing::error!("{}", e);
                channel
                    .say(ctx, format!("Failed to unmute '{}'", name))
                    .await?;
            } else {
                channel
                    .say(
                        ctx,
                        format!("The user '{}' is now unmuted for '{}'", name, game.name),
                    )
                    .await?;
            }
        }
    }
    Ok(())
}

async fn _subscribe(ctx: &Context, msg: &Message, mut args: Args, evts: Events) -> CommandResult {
    let channel_id = msg.channel_id;
    let guild_id = msg.guild_id;

    let filter = match args.single::<u32>() {
        Ok(id) => Id::eq(id),
        Err(_) => Fulltext::eq(args.single_quoted::<String>()?),
    };

    let data = ctx.data.read().await;
    let modio = data.get::<ModioKey>().expect("get modio failed");

    let game = modio.games().search(filter).first().await?;

    if let Some(game) = game {
        if !game
            .api_access_options
            .contains(ApiAccessOptions::ALLOW_THIRD_PARTY)
        {
            let msg = format!(
                ":no_entry: Third party API access is disabled for '{}' but is required for subscriptions.",
                game.name
            );
            let _ = channel_id.say(ctx, msg).await;
            return Ok(());
        }
        let data = ctx.data.read().await;
        let subs = data.get::<Subscriptions>().expect("get subs failed");
        let game_tags = game
            .tag_options
            .into_iter()
            .map(|opt| opt.tags)
            .flatten()
            .collect::<Tags>();

        let (hidden, mut sub_tags) = args
            .iter()
            .quoted()
            .flatten()
            .partition::<Tags, _>(|e| e.starts_with('*'));

        if !sub_tags.is_subset(&game_tags) {
            let mut msg = format!("Failed to subscribe to '{}'.\n", game.name);
            msg.push_str("Invalid tag(s): ");
            push_tags(&mut msg, sub_tags.difference(&game_tags));

            msg.push_str("\nAvailable tags: ");
            push_tags(&mut msg, game_tags.iter());

            channel_id.say(ctx, msg).await?;
            return Ok(());
        }

        sub_tags.extend(hidden);

        let ret = subs.add(game.id, channel_id, sub_tags, guild_id, evts);
        match ret {
            Ok(_) => {
                let _ = channel_id
                    .say(ctx, format!("Subscribed to '{}'", game.name))
                    .await;
            }
            Err(e) => tracing::error!("{}", e),
        }
    }
    Ok(())
}

async fn _unsubscribe(ctx: &Context, msg: &Message, mut args: Args, evts: Events) -> CommandResult {
    let channel_id = msg.channel_id;

    let data = ctx.data.read().await;
    let modio = data.get::<ModioKey>().expect("get modio failed");

    let filter = match args.single::<u32>() {
        Ok(id) => Id::eq(id),
        Err(_) => Fulltext::eq(args.single_quoted::<String>()?),
    };
    let game = modio.games().search(filter).first().await?;

    if let Some(game) = game {
        let data = ctx.data.read().await;
        let subs = data.get::<Subscriptions>().expect("get subs failed");
        let game_tags = game
            .tag_options
            .into_iter()
            .map(|opt| opt.tags)
            .flatten()
            .collect::<Tags>();

        let (hidden, mut sub_tags) = args
            .iter()
            .quoted()
            .flatten()
            .partition::<Tags, _>(|e| e.starts_with('*'));

        if !sub_tags.is_subset(&game_tags) {
            let mut msg = format!("Failed to unsubscribe from '{}'.\n", game.name);
            msg.push_str("Invalid tag(s): ");
            push_tags(&mut msg, sub_tags.difference(&game_tags));

            msg.push_str("\nAvailable tags: ");
            push_tags(&mut msg, game_tags.iter());

            channel_id.say(ctx, msg).await?;
            return Ok(());
        }

        sub_tags.extend(hidden);

        let ret = subs.remove(game.id, channel_id, sub_tags, evts);
        match ret {
            Ok(_) => {
                let _ = channel_id
                    .say(ctx, format!("Unsubscribed from '{}'", game.name))
                    .await;
            }
            Err(e) => tracing::error!("{}", e),
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
    let game = if let Some(game) = modio.games().search(game_filter).first().await? {
        game
    } else {
        return Ok((None, None));
    };

    let mod_ = modio
        .game(game.id)
        .mods()
        .search(mod_filter)
        .first()
        .await?;

    if let Some(mod_) = mod_ {
        Ok((Some(game), Some(mod_)))
    } else {
        Ok((Some(game), None))
    }
}

fn push_tags<'a, I>(s: &mut String, iter: I)
where
    I: std::iter::Iterator<Item = &'a String>,
{
    let mut iter = iter.peekable();
    while let Some(t) = iter.next() {
        s.push('`');
        s.push_str(&t);
        s.push('`');
        if iter.peek().is_some() {
            s.push_str(", ");
        }
    }
}
