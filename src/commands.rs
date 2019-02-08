use std::fmt::Write;

use futures::Future;
use modio::mods::ModsListOptions;
use serenity::framework::standard::CommandError;

use crate::util::{GameKey, Identifier};

type CommandResult = Result<(), CommandError>;

mod game;
mod info;

pub use game::Game;
pub use info::ModInfo;

command!(
    ListGames(self, _ctx, msg) {
        let channel = msg.channel_id;
        let task = self
            .modio
            .games()
            .list(&Default::default())
            .and_then(move |list| {
                let mut buf = String::new();
                for game in list {
                    let _ = writeln!(
                        &mut buf,
                        "{}. {}  *(name-id='{}')*",
                        game.id, game.name, game.name_id,
                    );
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
                    for m in list {
                        let _ = writeln!(&mut buf, "{}. {}", m.id, m.name);
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

command!(
    Search(self, ctx, msg, args) {
        let channel = msg.channel_id;
        let game_id = msg.guild_id.and_then(|id| {
            let data = ctx.data.lock();
            let map = data.get::<GameKey>().expect("failed to get map");
            map.get(&id).cloned()
        });
        if let Some(Identifier::Id(game_id)) = game_id {
            let term = args.single::<String>()?;
            let mut opts = ModsListOptions::new();
            opts.fulltext(term);

            let task = self
                .modio
                .game(game_id)
                .mods()
                .list(&opts)
                .and_then(move |list| {
                    if list.count == 0 {
                        let _ = channel.say("no mods found.");
                    }
                    let mut buf = String::new();
                    for m in list {
                        let _ = writeln!(&mut buf, "{}. {}", m.id, m.name);
                    }
                    dbg!(buf.len());
                    let _ = channel.say(buf);
                    Ok(())
                })
                .map_err(|e| {
                    eprintln!("{}", e);
                });

            self.executor.spawn(task);
        }
    }

    options(opts) {
        opts.desc = Some("Search mods".to_string());
        opts.usage = Some("search <arg>".to_string());
        opts.guild_only = true;
        opts.min_args = Some(1);
        opts.max_args = Some(1);
    }
);
