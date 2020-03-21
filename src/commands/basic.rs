use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::Args;
use serenity::model::channel::Message;

use crate::commands::CommandResult;
use crate::db::Settings;
use crate::util::guild_stats;

#[command]
#[description("Get bot info")]
pub async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    let (bot, name, avatar) = ctx
        .cache
        .current_user_field(|u| (*u.id.as_u64(), u.name.clone(), u.avatar_url()))
        .await;

    let dbl = if crate::tasks::dbl::is_dbl_enabled() {
        let profile = crate::tasks::dbl::get_profile(bot);
        let value = format!("[Profile]({}) | [Vote]({0}/vote)", profile);
        Some(("top.gg / discordbots.org", value, true))
    } else {
        None
    };
    let (guilds, users) = guild_stats(ctx).await;
    msg.channel_id
        .send_message(ctx, move |m| {
            m.embed(|e| {
                e.author(|a| {
                    let mut a = a.name(name);
                    if let Some(avatar) = avatar {
                        a = a.icon_url(avatar);
                    }
                    a
                })
                .footer(|f| f.text(format!("Servers: {} | Users: {}", guilds, users)))
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
                        "{} ([{}](https://github.com/nickelc/modio-bot/commit/{}))",
                        env!("CARGO_PKG_VERSION"),
                        env!("VERGEN_SHA_SHORT"),
                        env!("VERGEN_SHA"),
                    ),
                    true,
                )
                .fields(dbl)
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
#[description("Link to 'Getting Started' blog post.")]
#[aliases("tutorial", "getting-started")]
pub async fn guide(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id
        .say(ctx, "https://apps.mod.io/guides/getting-started")
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
#[description("Get link to vote for Modbot on Discord Bot List")]
pub async fn vote(ctx: &Context, msg: &Message) -> CommandResult {
    let bot = ctx.cache.current_user_field(|u| *u.id.as_u64()).await;
    let profile = crate::tasks::dbl::get_profile(bot);
    msg.channel_id.say(ctx, format!("{}/vote", profile)).await?;
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
                let _ = msg.react(ctx, '\u{01F44C}'); // :ok_hand:
            }
        }
        Err(e) => {
            eprintln!("Error sending server list: {:?}", e);
            let _ = msg
                .channel_id
                .say(ctx, "There was a problem sending you the server list.")
                .await;
        }
    }
    Ok(())
}
