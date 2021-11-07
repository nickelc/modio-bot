use modio::filter::prelude::*;
use modio::games::{ApiAccessOptions, Statistics};

use crate::commands::prelude::*;
use crate::util::ContentBuilder;

enum Identifier {
    Id(u32),
    Search(String),
}

#[command("games")]
#[description = "List all games on <https://mod.io>"]
#[bucket = "simple"]
#[max_args(0)]
pub async fn list_games(ctx: &Context, msg: &Message) -> CommandResult {
    let channel = msg.channel_id;
    let data = ctx.data.read().await;
    let modio = data.get::<ModioKey>().expect("get modio failed");

    let games = modio
        .games()
        .search(Default::default())
        .iter()
        .await?
        .try_fold(ContentBuilder::default(), |mut buf, game| {
            let _ = writeln!(&mut buf, "{}. {}", game.id, game.name);
            future::ok(buf)
        })
        .await?;

    for content in games {
        let ret = channel
            .send_message(ctx, |m| m.embed(|e| e.title("Games").description(content)))
            .await;
        if let Err(e) = ret {
            tracing::error!("{:?}", e);
        }
    }
    Ok(())
}

#[command]
#[description = "Display or set the default game."]
#[usage = "game [id|search]"]
#[bucket = "simple"]
#[min_args(0)]
#[only_in(guilds)]
pub async fn game(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let id = match args.single::<u32>() {
        Ok(id) => Some(Identifier::Id(id)),
        Err(ArgError::Parse(_)) => Some(Identifier::Search(args.rest().into())),
        Err(_) => None,
    };
    match id {
        Some(id) => set_game(ctx, msg, id).await,
        None => get_game(ctx, msg).await,
    }?;
    Ok(())
}

async fn get_game(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let settings = data.get::<Settings>().expect("get settings failed");
    let modio = data.get::<ModioKey>().expect("get modio failed");

    let channel = msg.channel_id;
    let game_id = msg.guild_id.and_then(|id| settings.game(id));

    if let Some(id) = game_id {
        let stats = modio.game(id).statistics();
        let (game, stats) = future::try_join(modio.game(id).get(), stats).await?;

        if let Err(e) = channel
            .send_message(ctx, |m| game.create_message(stats, m))
            .await
        {
            tracing::error!("{} {:?}", e, e);
        }
    }
    Ok(())
}

async fn set_game(ctx: &Context, msg: &Message, id: Identifier) -> CommandResult {
    let channel = msg.channel_id;

    if let Some(guild_id) = msg.guild_id {
        let game = {
            let data = ctx.data.read().await;
            let modio = data.get::<ModioKey>().expect("get modio failed");
            let filter = match id {
                Identifier::Id(id) => Id::eq(id),
                Identifier::Search(id) => Fulltext::eq(id),
            };
            modio.games().search(filter).first().await?
        };

        if let Some(game) = game {
            if !game
                .api_access_options
                .contains(ApiAccessOptions::ALLOW_THIRD_PARTY)
            {
                let msg = format!(
                    ":no_entry: Third party API access is disabled for '{}' but is required for the commands.",
                    game.name
                );
                let _ = channel.say(ctx, msg).await;
                return Ok(());
            }
            {
                let mut data = ctx.data.write().await;
                let settings = data.get_mut::<Settings>().expect("get settings failed");
                settings.set_game(guild_id, game.id)?;
            }
            let _ = channel
                .say(ctx, format!("Game is set to '{}'", game.name))
                .await;
        } else {
            let _ = channel.say(ctx, "Game not found").await;
        }
    }
    Ok(())
}

trait GameExt {
    fn create_fields(&self, _: Statistics) -> Vec<EmbedField>;

    fn create_message<'a, 'b>(
        &self,
        _: Statistics,
        m: &'b mut CreateMessage<'a>,
    ) -> &'b mut CreateMessage<'a>;
}

impl GameExt for modio::games::Game {
    fn create_fields(&self, s: Statistics) -> Vec<EmbedField> {
        fn info(g: &modio::games::Game) -> EmbedField {
            (
                "Info",
                format!(
                    r#"**Id:** {}
**Name-Id:** {}
**Profile:** {}"#,
                    g.id, g.name_id, g.profile_url,
                ),
                true,
            )
        }
        fn stats(stats: Statistics) -> EmbedField {
            let total = stats.mods_total;
            let subs = stats.subscribers_total;
            let downloads = stats.downloads.total;
            (
                "Stats",
                format!(
                    r#"**Mods:** {}
**Subscribers:** {}
**Downloads:** {}"#,
                    total, subs, downloads,
                ),
                true,
            )
        }
        vec![info(self), stats(s)]
    }

    fn create_message<'a, 'b>(
        &self,
        stats: Statistics,
        m: &'b mut CreateMessage<'a>,
    ) -> &'b mut CreateMessage<'a> {
        m.embed(|e| {
            e.title(self.name.to_string())
                .url(self.profile_url.to_string())
                .author(|a| self.submitted_by.create_author(a))
                .description(self.summary.to_string())
                .image(self.logo.thumb_640x360.to_string())
                .fields(self.create_fields(stats))
        });
        m
    }
}
