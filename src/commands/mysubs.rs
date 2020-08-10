use std::collections::BTreeMap;

use modio::filter::prelude::*;

use crate::commands::prelude::*;
use crate::db::Users;
use crate::util::ContentBuilder;

#[command]
#[description = "List your subscriptions"]
#[sub_commands(mysubs_add, mysubs_rm)]
pub async fn mysubs(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let users = data.get::<Users>().expect("get users failed");
    if let Some(token) = users.find_token(msg.author.id)? {
        let modio = data.get::<ModioKey>().expect("get modio failed").clone();
        let modio = modio.with_token(token);

        let filter = Default::default();
        let it = modio.user().subscriptions(filter).iter().await?;

        let mut mods = it
            .try_fold(BTreeMap::new(), |mut mods, m| async {
                mods.entry(m.game_id).or_insert_with(Vec::new).push(m);
                Ok(mods)
            })
            .await?;

        let ids: Vec<_> = mods.keys().collect();
        let games = modio.games().search(Id::_in(ids)).collect().await?;

        let mut mapped = BTreeMap::new();
        for game in games {
            if let Some(mods) = mods.remove(&game.id) {
                mapped.insert(game.name, mods);
            }
        }

        let mods = mapped;
        let mut content = ContentBuilder::default();
        for (game, mods) in mods {
            let _ = writeln!(&mut content, "**{}**", game);
            for m in mods {
                let _ = content.write_str(&format!(
                    "{:02}. [{}]({}) ({}) +{}/-{}\n",
                    m.stats.popularity.rank_position,
                    m.name,
                    m.profile_url,
                    m.id,
                    m.stats.ratings.positive,
                    m.stats.ratings.negative,
                ));
            }
        }
        for content in content {
            let ret = msg
                .channel_id
                .send_message(ctx, |m| {
                    m.embed(|e| e.title("My subscriptions").description(content))
                })
                .await;
            if let Err(e) = ret {
                eprintln!("{:?}", e);
            }
        }
    } else {
        msg.channel_id
            .say(ctx, "modbot is not authorized to access your subscriptions")
            .await?;
    }
    Ok(())
}

#[command("add")]
pub async fn mysubs_add(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let game_filter = match args.single::<u32>() {
        Ok(id) => Id::eq(id),
        Err(_) => Fulltext::eq(args.quoted().single::<String>()?),
    };
    let mod_filter = match args.single::<u32>() {
        Ok(id) => Id::eq(id),
        Err(_) => Fulltext::eq(args.quoted().single::<String>()?),
    };

    let data = ctx.data.read().await;
    let users = data.get::<Users>().expect("get users failed");
    if let Some(token) = users.find_token(msg.author.id)? {
        let modio = data.get::<ModioKey>().expect("get modio failed").clone();
        let modio = modio.with_token(token);

        let (game, mod_) = find_game_mod(modio.clone(), game_filter, mod_filter).await?;
        let game_mod = match (&game, &mod_) {
            (Some(g), Some(m)) => {
                let ret = modio.mod_(g.id, m.id).subscribe().await;
                (game, mod_, ret)
            }
            _ => (game, mod_, Ok(())),
        };

        let channel = msg.channel_id;
        match game_mod {
            (None, _, _) => {
                channel.say(ctx, "Game not found").await?;
            }
            (_, None, _) => {
                channel.say(ctx, "Mod not found").await?;
            }
            (_, Some(mod_), Err(e)) => {
                eprintln!("{}", e);
                channel
                    .say(ctx, format!("Failed to subscribe to '{}'", mod_.name))
                    .await?;
            }
            (Some(_game), Some(mod_), Ok(_)) => {
                channel
                    .say(ctx, format!("Subscribed to '{}'", mod_.name))
                    .await?;
            }
        }
    } else {
        msg.channel_id
            .say(ctx, "modbot is not authorized to manage your subscriptions")
            .await?;
    }
    Ok(())
}

#[command("rm")]
pub async fn mysubs_rm(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let game_filter = match args.single::<u32>() {
        Ok(id) => Id::eq(id),
        Err(_) => Fulltext::eq(args.quoted().single::<String>()?),
    };
    let mod_filter = match args.single::<u32>() {
        Ok(id) => Id::eq(id),
        Err(_) => Fulltext::eq(args.quoted().single::<String>()?),
    };

    let data = ctx.data.read().await;
    let users = data.get::<Users>().expect("get users failed");
    if let Some(token) = users.find_token(msg.author.id)? {
        let modio = data.get::<ModioKey>().expect("get modio failed").clone();
        let modio = modio.with_token(token);

        let (game, mod_) = find_game_mod(modio.clone(), game_filter, mod_filter).await?;
        let game_mod = match (&game, &mod_) {
            (Some(g), Some(m)) => {
                let ret = modio.mod_(g.id, m.id).unsubscribe().await;
                (game, mod_, ret)
            }
            _ => (game, mod_, Ok(())),
        };

        let channel = msg.channel_id;
        match game_mod {
            (None, _, _) => {
                channel.say(ctx, "Game not found").await?;
            }
            (_, None, _) => {
                channel.say(ctx, "Mod not found").await?;
            }
            (_, Some(mod_), Err(e)) => {
                eprintln!("{}", e);
                channel
                    .say(ctx, format!("Failed to unsubscribe from '{}'", mod_.name))
                    .await?;
            }
            (Some(_game), Some(mod_), Ok(_)) => {
                channel
                    .say(ctx, format!("Unsubscribed from '{}'", mod_.name))
                    .await?;
            }
        }
    } else {
        msg.channel_id
            .say(ctx, "modbot is not authorized to manage your subscriptions")
            .await?;
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
