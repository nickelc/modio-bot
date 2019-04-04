use std::sync::Arc;

use serenity::client::Context;
use serenity::framework::standard::{Args, Command, CommandOptions};
use serenity::model::channel::Message;
use serenity::model::permissions::Permissions;

use crate::commands::CommandResult;
use crate::db::Settings;

pub struct About;

impl Command for About {
    fn options(&self) -> Arc<CommandOptions> {
        Arc::new(CommandOptions {
            desc: Some("Get bot info.".to_string()),
            ..Default::default()
        })
    }

    fn execute(&self, _: &mut Context, msg: &Message, _: Args) -> CommandResult {
        serenity::http::raw::get_current_user().and_then(|u| {
            msg.channel_id.send_message(|m| {
                m.embed(|e| {
                    e.author(|a| {
                        let mut a = a.name(&u.name);
                        if let Some(avatar) = u.avatar_url() {
                            a = a.icon_url(&avatar);
                        }
                        a
                    })
                    .field("Invite", "[discordbot.mod.io](https://discordbot.mod.io)", true)
                    .field("Website", "[mod.io](https://mod.io)", true)
                    .field(
                        "mod.io Discord",
                        "[discord.mod.io](https://discord.mod.io)",
                        true,
                    )
                    .field(
                        "modbot Discord",
                        "[discord.gg/4akZJFf](https://discord.gg/4akZJFf)",
                        true,
                    )
                    .field(
                        "Version",
                        format!(
                            "{} ({})",
                            env!("CARGO_PKG_VERSION"),
                            env!("VERGEN_SHA_SHORT"),
                        ),
                        true,
                    )
                    .field(
                        "Github",
                        "[nickelc/modio-bot](https://github.com/nickelc/modio-bot)",
                        true,
                    )
                })
            })
        })?;
        Ok(())
    }
}

pub struct Invite;

impl Command for Invite {
    fn options(&self) -> Arc<CommandOptions> {
        Arc::new(CommandOptions {
            desc: Some("Displays a link to invite modbot.".to_string()),
            ..Default::default()
        })
    }

    fn execute(&self, _: &mut Context, msg: &Message, _: Args) -> CommandResult {
        let _ = msg.channel_id.say("Visit <https://discordbot.mod.io> to invite modbot.");
        Ok(())
    }
}

pub struct Guide;

impl Command for Guide {
    fn options(&self) -> Arc<CommandOptions> {
        Arc::new(CommandOptions {
            desc: Some("Link to 'Getting Started' blog post.".to_string()),
            aliases: vec!["tutorial".to_string(), "getting-started".to_string()],
            ..Default::default()
        })
    }

    fn execute(&self, _: &mut Context, msg: &Message, _: Args) -> CommandResult {
        msg.channel_id
            .say("https://apps.mod.io/guides/getting-started")?;
        Ok(())
    }
}

pub struct Prefix;

impl Command for Prefix {
    fn options(&self) -> Arc<CommandOptions> {
        Arc::new(CommandOptions {
            desc: Some("Set prefix for server".to_string()),
            guild_only: true,
            max_args: Some(1),
            required_permissions: Permissions::MANAGE_GUILD,
            ..Default::default()
        })
    }

    fn execute(&self, ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
        let prefix = args.single::<String>().ok();
        match &prefix {
            Some(prefix) => msg
                .channel_id
                .say(format!("Prefix is set to `{}`", prefix))?,
            None => msg.channel_id.say("Prefix is set to `~`")?,
        };
        Settings::set_prefix(ctx, msg.guild_id.expect("guild only"), prefix);
        Ok(())
    }
}
