use futures::Future;
use modio::filter::Operator;
use modio::games::GamesListOptions;
use modio::users::User;
use modio::Connect;
use serenity::builder::{CreateEmbedAuthor, CreateMessage};
use serenity::client::Context;
use serenity::model::channel::Message;

use crate::commands::CommandResult;
use crate::util::{GameKey, Identifier};

type EmbedField = (&'static str, String, bool);
type Stats = (u32, u32, u32);

command!(
    Game(self, ctx, msg, args) {
        match args.single::<Identifier>().ok() {
            Some(id) => self.set_game(ctx, msg, id),
            None => self.game(ctx, msg),
        }?;
    }

    options(opts) {
        opts.help_available = true;
        opts.desc = Some("Display or set the default game.".to_string());
        opts.usage = Some("game [id|name-id]".to_string());
        opts.guild_only = true;
        opts.min_args = Some(0);
        opts.max_args = Some(1);
    }
);

impl<C> Game<C>
where
    C: Clone + Connect + 'static,
{
    fn game(&self, ctx: &mut Context, msg: &Message) -> CommandResult {
        let channel = msg.channel_id;
        let game_id = msg.guild_id.and_then(|id| {
            let data = ctx.data.lock();
            let map = data.get::<GameKey>().expect("failed to get map");
            map.get(&id).cloned()
        });

        if let Some(Identifier::Id(id)) = game_id {
            let stats = self
                .modio
                .game(id)
                .mods()
                .list(&Default::default())
                .and_then(|list| {
                    let total = list.total;
                    Ok(list
                        .into_iter()
                        .fold((total, 0, 0), |(total, mut dl, mut sub), m| {
                            dl += m.stats.downloads_total;
                            sub += m.stats.subscribers_total;
                            (total, dl, sub)
                        }))
                });
            let task = self
                .modio
                .game(id)
                .get()
                .join(stats)
                .and_then(move |(game, stats)| {
                    if let Err(e) = channel.send_message(|m| game.create_message(stats, m)) {
                        eprintln!("{} {:?}", e, e);
                    }
                    Ok(())
                })
                .map_err(|e| {
                    eprintln!("{}", e);
                });
            self.executor.spawn(task);
        }
        Ok(())
    }

    fn set_game(&self, ctx: &mut Context, msg: &Message, id: Identifier) -> CommandResult {
        let ctx2 = ctx.clone();
        let channel = msg.channel_id;

        if let Some(guild_id) = msg.guild_id {
            let mut opts = GamesListOptions::new();
            match id {
                Identifier::Id(id) => opts.id(Operator::Equals, id),
                Identifier::NameId(id) => opts.name_id(Operator::Equals, id),
            };
            let task = self
                .modio
                .games()
                .list(&opts)
                .and_then(|mut list| Ok(list.shift()))
                .and_then(move |game| {
                    if let Some(game) = game {
                        let mut data = ctx2.data.lock();
                        let map = data.get_mut::<GameKey>().unwrap();
                        map.insert(guild_id, Identifier::Id(game.id));
                        let _ = channel.say(format!("Game is set to '{}'", game.name));
                    } else {
                        let _ = channel.say("Game not found");
                    }
                    Ok(())
                })
                .map_err(|e| {
                    eprintln!("{}", e);
                });
            self.executor.spawn(task);
        }
        Ok(())
    }
}

trait UserExt {
    fn create_author(&self, _: CreateEmbedAuthor) -> CreateEmbedAuthor;
}

impl UserExt for User {
    fn create_author(&self, mut a: CreateEmbedAuthor) -> CreateEmbedAuthor {
        a = a.name(&self.username).url(&self.profile_url.to_string());
        if let Some(avatar) = &self.avatar {
            let icon = avatar.original.to_string();
            a = a.icon_url(&icon);
        }
        a
    }
}

trait GameExt {
    fn create_fields(&self, _: Stats) -> Vec<EmbedField>;

    fn create_message(&self, _: Stats, m: CreateMessage) -> CreateMessage;
}

impl GameExt for modio::games::Game {
    fn create_fields(&self, s: Stats) -> Vec<EmbedField> {
        fn info(g: &modio::games::Game) -> EmbedField {
            (
                "Info",
                format!(
                    r#"**Id:** {}
**Name-Id:** {}
**Profile:** {}"#,
                    g.id,
                    g.name_id,
                    g.profile_url.to_string(),
                ),
                true,
            )
        }
        fn stats(stats: Stats) -> EmbedField {
            let (total, downloads, subs) = stats;
            (
                "Stats",
                format!(
                    r#"**Mods:** {}
**Subscribers:** {}
**Downloads:** {}"#,
                    total, downloads, subs,
                ),
                true,
            )
        }
        vec![info(self), stats(s)]
    }

    fn create_message(&self, stats: Stats, m: CreateMessage) -> CreateMessage {
        m.embed(|e| {
            e.title(self.name.to_string())
                .url(self.profile_url.to_string())
                .author(|a| self.submitted_by.create_author(a))
                .description(self.summary.to_string())
                .image(self.logo.thumb_640x360.to_string())
                .fields(self.create_fields(stats))
        })
    }
}
