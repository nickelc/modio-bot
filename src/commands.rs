use std::collections::HashSet;

use modio::user::User;
use serenity::builder::{CreateEmbedAuthor, CreateEmbedFooter};
use serenity::client::Context;
use serenity::framework::standard::macros::{group, help, hook};
use serenity::framework::standard::{
    help_commands, Args, CommandGroup, CommandResult, DispatchError, HelpOptions,
};
use serenity::model::prelude::*;

pub type EmbedField = (&'static str, String, bool);

pub mod prelude {
    pub use std::fmt::Write;
    pub use std::future::Future;

    pub use futures_core::Stream;
    pub use futures_util::{future, StreamExt, TryFutureExt, TryStreamExt};
    pub use modio::filter::Operator;
    pub use modio::user::User;
    pub use serenity::builder::{CreateEmbedAuthor, CreateMessage};
    pub use serenity::client::Context;
    pub use serenity::framework::standard::macros::command;
    pub use serenity::framework::standard::{ArgError, Args, CommandResult};
    pub use serenity::model::channel::Message;
    pub use serenity::model::id::ChannelId;

    pub use super::{EmbedField, UserExt};
    pub use crate::bot::ModioKey;
    pub use crate::db::Settings;
    pub use crate::error::Error;
    pub use crate::util::format_timestamp;
}

mod basic;
mod game;
pub mod mods;
mod subs;

use crate::metrics::Metrics;

use basic::*;
use game::*;
use mods::*;
use subs::*;

#[group]
#[commands(servers)]
struct Owner;

#[group]
#[commands(about, prefix, invite)]
struct General;

#[group]
#[commands(list_games, game, list_mods, mod_info, popular)]
struct Basic;

#[group]
#[commands(
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
#[required_permissions("MANAGE_CHANNELS")]
struct Subscriptions;

#[hook]
pub async fn before(_: &Context, msg: &Message, _: &str) -> bool {
    tracing::debug!("cmd: {:?}: {:?}: {}", msg.guild_id, msg.author, msg.content);
    true
}

#[hook]
pub async fn after(ctx: &Context, _: &Message, name: &str, result: CommandResult) {
    let data = ctx.data.read().await;
    let metrics = data.get::<Metrics>().expect("get metrics failed");
    metrics.commands.total.inc();
    metrics.commands.counts.with_label_values(&[name]).inc();
    if result.is_err() {
        metrics.commands.errored.inc();
    }
}

#[hook]
pub async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError) -> () {
    match error {
        DispatchError::NotEnoughArguments { .. } => {
            let _ = msg.channel_id.say(ctx, "Not enough arguments.").await;
        }
        DispatchError::CommandDisabled(_) => {
            let _ = msg
                .channel_id
                .say(ctx, "The command is currently disabled.")
                .await;
        }
        DispatchError::LackingPermissions(_) => {
            let _ = msg
                .channel_id
                .say(ctx, "You have insufficient rights for this command, you need the `MANAGE_CHANNELS` permission.")
                .await;
        }
        DispatchError::Ratelimited(_) => {
            let _ = msg.channel_id.say(ctx, "Try again in 1 second.").await;
        }
        e => tracing::error!("Dispatch error: {:?}", e),
    }
}

#[help]
async fn help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
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
