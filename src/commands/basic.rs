use std::sync::Arc;

use serenity::client::Context;
use serenity::framework::standard::{Args, Command, CommandOptions};
use serenity::model::channel::Message;
use serenity::model::permissions::Permissions;
use serenity::CACHE;

use crate::commands::CommandResult;
use crate::db::Settings;
use crate::util::guild_stats;

pub struct About;

impl Command for About {
    fn options(&self) -> Arc<CommandOptions> {
        Arc::new(CommandOptions {
            desc: Some("Get bot info.".to_string()),
            ..Default::default()
        })
    }

    fn execute(&self, ctx: &mut Context, msg: &Message, _: Args) -> CommandResult {
        serenity::http::raw::get_current_user().and_then(|u| {
            let dbl = if crate::dbl::is_dbl_enabled() {
                let profile = crate::dbl::get_profile();
                let value = format!("[Profile]({}) | [Vote]({0}/vote)", profile);
                Some(("discordbots.org", value, true))
            } else {
                None
            };
            msg.channel_id.send_message(|m| {
                m.embed(|e| {
                    e.author(|a| {
                        let mut a = a.name(&u.name);
                        if let Some(avatar) = u.avatar_url() {
                            a = a.icon_url(&avatar);
                        }
                        a
                    })
                    .footer(|f| {
                        let (guilds, users) = guild_stats(ctx);
                        f.text(format!("Servers: {} | Users: {}", guilds, users))
                    })
                    .field(
                        "Invite to server",
                        "[discordbot.mod.io](https://discordbot.mod.io)",
                        true,
                    )
                    .field("Website", "[mod.io](https://mod.io)", true)
                    .field(
                        "mod.io Discord",
                        "[discord.mod.io](https://discord.mod.io)",
                        true,
                    )
                    .field(
                        "modbot Discord",
                        "[discord.gg/XNX9665](https://discord.gg/XNX9665)",
                        true,
                    )
                    .field(
                        "Github",
                        "[nickelc/modio-bot](https://github.com/nickelc/modio-bot)",
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
                    .fields(dbl)
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
        let _ = msg
            .channel_id
            .say("Visit <https://discordbot.mod.io> to invite modbot to join your Discord server. Once modbot has joined, you can set the default game and subscribe to game(s) for updates using the `game` and `subscribe` commands.");
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

pub struct Vote;

impl Command for Vote {
    fn options(&self) -> Arc<CommandOptions> {
        Arc::new(CommandOptions {
            desc: Some("Get link to vote for Modbot on Discord Bot List".to_string()),
            ..Default::default()
        })
    }

    fn execute(&self, _: &mut Context, msg: &Message, _: Args) -> CommandResult {
        let profile = crate::dbl::get_profile();
        msg.channel_id.say(format!("{}/vote", profile))?;
        Ok(())
    }
}

pub struct Servers;

impl Command for Servers {
    fn options(&self) -> Arc<CommandOptions> {
        Arc::new(CommandOptions {
            owners_only: true,
            dm_only: true,
            help_available: false,
            ..Default::default()
        })
    }

    fn execute(&self, _: &mut Context, msg: &Message, _: Args) -> CommandResult {
        use std::fmt::Write;

        let buf = CACHE
            .read()
            .guilds
            .values()
            .fold(String::new(), |mut buf, guild| {
                let guild = guild.read();
                let _ = writeln!(
                    &mut buf,
                    "- {} (id: {}, members: {})",
                    guild.name,
                    guild.id,
                    guild.members.len(),
                );
                buf
            });
        match msg.author.direct_message(move |m| m.content(buf)) {
            Ok(_) => {
                if msg.guild_id.is_some() {
                    let _ = msg.react('\u{01F44C}'); // :ok_hand:
                }
            }
            Err(e) => {
                eprintln!("Error sending server list: {:?}", e);
                let _ = msg
                    .channel_id
                    .say("There was a problem sending you the server list.");
            }
        }
        Ok(())
    }
}
