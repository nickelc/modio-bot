use std::collections::HashSet;

use modio::user::User;
use serenity::builder::{CreateEmbedAuthor, CreateEmbedFooter};
use serenity::client::Context;
use serenity::framework::standard::macros::{group, help};
use serenity::framework::standard::{
    help_commands, Args, CommandGroup, CommandResult, HelpOptions,
};
use serenity::model::prelude::*;

pub type EmbedField = (&'static str, String, bool);

pub mod prelude {
    pub use std::fmt::Write;

    pub use futures::{Future, Stream};
    pub use modio::filter::Operator;
    pub use modio::user::User;
    pub use serenity::builder::{CreateEmbedAuthor, CreateMessage};
    pub use serenity::client::Context;
    pub use serenity::framework::standard::macros::command;
    pub use serenity::framework::standard::{ArgError, Args, CommandResult};
    pub use serenity::model::channel::Message;
    pub use serenity::model::id::ChannelId;

    pub use super::{EmbedField, UserExt};
    pub use crate::bot::{ExecutorKey, ModioKey};
    pub use crate::db::Settings;
    pub use crate::error::Error;
    pub use crate::util::format_timestamp;
}

mod basic;
mod game;
pub mod mods;
mod subs;

use basic::*;
use game::*;
use mods::*;
use subs::*;

#[group]
#[commands(servers)]
struct Owner;

#[group]
#[commands(about, prefix, invite, guide)]
struct General;

#[group]
#[commands(
    list_games,
    game,
    list_mods,
    mod_info,
    popular,
    subscriptions,
    subscribe,
    unsubscribe,
    muted,
    mute,
    unmute,
    muted_users,
    mute_user,
    unmute_user
)]
struct Modio;

pub mod with_vote {
    use super::*;

    #[group]
    #[commands(about, prefix, invite, guide, vote)]
    struct General;
}

#[help]
fn help(
    context: &mut Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners)
}

pub trait UserExt {
    fn create_author<'a>(&self, _: &'a mut CreateEmbedAuthor) -> &'a mut CreateEmbedAuthor;

    fn create_footer<'a>(&self, _: &'a mut CreateEmbedFooter) -> &'a mut CreateEmbedFooter;
}

impl UserExt for User {
    fn create_author<'a>(&self, mut a: &'a mut CreateEmbedAuthor) -> &'a mut CreateEmbedAuthor {
        a = a.name(&self.username).url(&self.profile_url.to_string());
        if let Some(avatar) = &self.avatar {
            let icon = avatar.original.to_string();
            a = a.icon_url(&icon);
        }
        a
    }

    fn create_footer<'a>(&self, mut f: &'a mut CreateEmbedFooter) -> &'a mut CreateEmbedFooter {
        f = f.text(&self.username);
        if let Some(avatar) = &self.avatar {
            f = f.icon_url(&avatar.thumb_50x50.to_string());
        }
        f
    }
}
