use std::fmt::Write;

use futures::Future;
use modio::filter::Operator;
use modio::games::GamesListOptions;
use modio::Connect;
use serenity::client::Context;
use serenity::framework::standard::CommandError;
use serenity::model::channel::Message;

use crate::util::{GameKey, Identifier};

type CommandResult = Result<(), CommandError>;

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
            let task = self
                .modio
                .game(id)
                .get()
                .and_then(move |game| {
                    let _ = channel.say(format!("Game: {}", game.name));
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
command!(
    ListGames(self, _ctx, msg) {
        let channel = msg.channel_id;
        let task = self
            .modio
            .games()
            .list(&Default::default())
            .and_then(move |list| {
                let mut buf = String::new();
                for (n, game) in list.into_iter().enumerate() {
                    let _ = writeln!(&mut buf, "{:02}. {}", n + 1, game.name);
                }
                let _ = channel.say(buf);
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
    ListMods(self, ctx, msg) {
        let channel = msg.channel_id;
        let game_id = msg.guild_id.and_then(|id| {
            let data = ctx.data.lock();
            let map = data.get::<GameKey>().expect("failed to get map");
            map.get(&id).cloned()
        });
        if let Some(Identifier::Id(id)) = game_id {
            let task = self
                .modio
                .game(id)
                .mods()
                .list(&Default::default())
                .and_then(move |list| {
                    if list.count == 0 {
                        let _ = channel.say("no mods found.");
                    }
                    let mut buf = String::new();
                    for (n, m) in list.into_iter().enumerate() {
                        let _ = writeln!(&mut buf, "{:02}. {}", n + 1, m.name);
                    }
                    let _ = channel.say(buf);
                    Ok(())
                })
                .map_err(|e| {
                    eprintln!("{}", e);
                });

            self.executor.spawn(task);
        } else {
            let _ = channel.say("default game is not set.");
        }
    }

    options(opts) {
        opts.desc = Some("List mods of the default game".to_string());
        opts.usage = Some("mods".to_string());
        opts.guild_only = true;
        opts.max_args = Some(0);
    }
);
