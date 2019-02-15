use modio::games::GamesListOptions;

use crate::commands::prelude::*;

type Stats = (usize, u32, u32);

command!(
    ListGames(self, _ctx, msg) {
        let channel = msg.channel_id;
        let task = self
            .modio
            .games()
            .list(&Default::default())
            .and_then(move |list| {
                let ret = channel.send_message(|m| list.create_message(m));
                if let Err(e) = ret {
                    eprintln!("{:?}", e);
                }
                Ok(())
            })
            .map_err(|e| {
                eprintln!("{}", e);
            });
        self.executor.spawn(task);
    }

    options(opts) {
        opts.help_available = true;
        opts.desc = Some("List all games on <https://mod.io>".to_string());
        opts.max_args = Some(0);
    }
);

command!(
    Game(self, ctx, msg, args) {
        let id = match args.single::<u32>() {
            Ok(id) => Some(Identifier::Id(id)),
            Err(ArgError::Parse(_)) => Some(Identifier::Search(args.rest().into())),
            Err(ArgError::Eos) => None,
        };
        match id {
            Some(id) => self.set_game(ctx, msg, id),
            None => self.game(ctx, msg),
        }?;
    }

    options(opts) {
        opts.help_available = true;
        opts.desc = Some("Display or set the default game.".to_string());
        opts.usage = Some("game [id|search]".to_string());
        opts.guild_only = true;
        opts.min_args = Some(0);
    }
);

impl<C> Game<C>
where
    C: Clone + Connect + 'static,
{
    fn game(&self, ctx: &mut Context, msg: &Message) -> CommandResult {
        let channel = msg.channel_id;
        let game_id = msg.guild_id.and_then(|id| Settings::game(ctx, id));

        if let Some(id) = game_id {
            let stats = self
                .modio
                .game(id)
                .mods()
                .statistics(&Default::default())
                .collect()
                .and_then(|list| {
                    let total = list.len();
                    Ok(list
                        .into_iter()
                        .fold((total, 0, 0), |(total, mut dl, mut sub), s| {
                            dl += s.downloads_total;
                            sub += s.subscribers_total;
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
        let mut ctx2 = ctx.clone();
        let channel = msg.channel_id;

        if let Some(guild_id) = msg.guild_id {
            let mut opts = GamesListOptions::new();
            match id {
                Identifier::Id(id) => opts.id(Operator::Equals, id),
                Identifier::Search(id) => opts.fulltext(id),
            };
            let task = self
                .modio
                .games()
                .list(&opts)
                .and_then(|mut list| Ok(list.shift()))
                .and_then(move |game| {
                    if let Some(game) = game {
                        Settings::set_game(&mut ctx2, guild_id, game.id);
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
                    total, subs, downloads,
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

impl ModioListResponseExt for ModioListResponse<modio::games::Game> {
    fn create_message(&self, m: CreateMessage) -> CreateMessage {
        let mut buf = String::new();
        for g in &self.data {
            let _ = writeln!(&mut buf, "{}. {}", g.id, g.name);
        }
        m.embed(|e| e.description(buf))
    }
}
