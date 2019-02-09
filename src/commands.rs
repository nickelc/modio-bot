use std::fmt::Write;

use futures::Future;
use modio::users::User;
use serenity::builder::CreateEmbedAuthor;
use serenity::framework::standard::CommandError;

use crate::util::{GameKey, Identifier};

pub type CommandResult = Result<(), CommandError>;
pub type EmbedField = (&'static str, String, bool);

pub mod prelude {
    pub use std::fmt::Write;

    pub use futures::Future;
    pub use modio::filter::Operator;
    pub use modio::users::User;
    pub use modio::Connect;
    pub use modio::ModioListResponse;
    pub use serenity::builder::{CreateEmbedAuthor, CreateMessage};
    pub use serenity::client::Context;
    pub use serenity::framework::standard::ArgError;
    pub use serenity::model::channel::Message;

    pub use super::{CommandResult, EmbedField, UserExt};
    pub use crate::util::{format_timestamp, GameKey, Identifier};
}

mod game;
mod info;

pub use game::Game;
pub use info::ModInfo;

pub trait UserExt {
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
