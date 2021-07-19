use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::Args;
use serenity::model::channel::Message;

use crate::commands::CommandResult;
use crate::db::Settings;

#[command]
#[description("Get bot info")]
pub async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    let (name, avatar) = ctx
        .cache
        .current_user_field(|u| (u.name.clone(), u.avatar_url()))
        .await;

    let guilds = ctx.cache.guild_count().await;
    msg.channel_id
        .send_message(ctx, move |m| {
            m.embed(|e| {
                let version = if env!("GIT_SHA") == "UNKNOWN" {
                    env!("CARGO_PKG_VERSION").to_string()
                } else {
                    format!(
                        "{} ([{}](https://github.com/nickelc/modio-bot/commit/{}))",
                        env!("CARGO_PKG_VERSION"),
                        env!("GIT_SHA_SHORT"),
                        env!("GIT_SHA"),
                    )
                };
                e.author(|a| {
                    let mut a = a.name(name);
                    if let Some(avatar) = avatar {
                        a = a.icon_url(avatar);
                    }
                    a
                })
                .footer(|f| f.text(format!("Servers: {}", guilds)))
                .field(
                    "Invite to server",
                    "[discordbot.mod.io](https://discordbot.mod.io)",
                    true,
                )
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
                    "Website/Blog",
                    "[ModBot for Discord](https://mod.io/blog/modbot-for-discord)",
                    true,
                )
                .field(
                    "Github",
                    "[nickelc/modio-bot](https://github.com/nickelc/modio-bot)",
                    true,
                )
                .field("Version", version, true)
            })
        })
        .await?;
    Ok(())
}

#[command]
#[description("Displays a link to invite modbot.")]
pub async fn invite(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id
        .say(ctx, "Visit <https://discordbot.mod.io> to invite modbot to join your Discord server. Once modbot has joined, you can set the default game and subscribe to game(s) for updates using the `game` and `subscribe` commands.")
        .await?;
    Ok(())
}

#[command]
#[description("Set prefix for the server")]
#[max_args(1)]
#[only_in(guilds)]
#[required_permissions("MANAGE_GUILD")]
pub async fn prefix(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let prefix = args.single::<String>().ok();
    match &prefix {
        Some(prefix) => {
            msg.channel_id
                .say(ctx, format!("Prefix is set to `{}`", prefix))
                .await?
        }
        None => msg.channel_id.say(ctx, "Prefix is set to `~`").await?,
    };
    let mut data = ctx.data.write().await;
    let settings = data.get_mut::<Settings>().expect("get settings failed");
    settings.set_prefix(msg.guild_id.expect("guild only"), prefix)?;
    Ok(())
}

#[command]
#[owners_only]
#[help_available(false)]
pub async fn servers(ctx: &Context, msg: &Message) -> CommandResult {
    use std::fmt::Write;

    let buf = {
        let guilds = ctx.cache.guilds().await;
        let mut buf = String::new();
        for id in guilds {
            let info = ctx
                .cache
                .guild_field(id, |g| (g.name.clone(), g.members.len()))
                .await;
            if let Some((name, members)) = info {
                let _ = writeln!(&mut buf, "- {} (id: {}, members: {})", name, id, members);
            }
        }
        buf
    };
    match msg
        .author
        .direct_message(ctx, move |m| m.content(buf))
        .await
    {
        Ok(_) => {
            if msg.guild_id.is_some() {
                let _ = msg.react(ctx, '\u{01F44C}').await; // :ok_hand:
            }
        }
        Err(e) => {
            tracing::error!("Error sending server list: {:?}", e);
            let _ = msg
                .channel_id
                .say(ctx, "There was a problem sending you the server list.")
                .await;
        }
    }
    Ok(())
}
