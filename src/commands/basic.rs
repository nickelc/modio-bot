use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::Args;
use serenity::model::channel::Message;

use crate::commands::CommandResult;
use crate::db::Settings;
use crate::util::guild_stats;

#[command]
#[description("Get bot info")]
pub fn about(ctx: &mut Context, msg: &Message) -> CommandResult {
    let bot = *ctx.cache.read().user.id.as_u64();
    let name = ctx.cache.read().user.name.to_owned();
    let avatar = ctx.cache.read().user.avatar_url();

    let dbl = if crate::dbl::is_dbl_enabled() {
        let profile = crate::dbl::get_profile(bot);
        let value = format!("[Profile]({}) | [Vote]({0}/vote)", profile);
        Some(("top.gg / discordbots.org", value, true))
    } else {
        None
    };
    let (guilds, users) = guild_stats(ctx);
    msg.channel_id.send_message(ctx, move |m| {
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
    })?;
    Ok(())
}

#[command]
#[description("Displays a link to invite modbot.")]
pub fn invite(ctx: &mut Context, msg: &Message) -> CommandResult {
    msg.channel_id
        .say(ctx, "Visit <https://discordbot.mod.io> to invite modbot to join your Discord server. Once modbot has joined, you can set the default game and subscribe to game(s) for updates using the `game` and `subscribe` commands.")?;
    Ok(())
}

#[command]
#[description("Link to 'Getting Started' blog post.")]
#[aliases("tutorial", "getting-started")]
pub fn guide(ctx: &mut Context, msg: &Message) -> CommandResult {
    msg.channel_id
        .say(ctx, "https://apps.mod.io/guides/getting-started")?;
    Ok(())
}

#[command]
#[description("Set prefix for the server")]
#[max_args(1)]
#[only_in(guilds)]
#[required_permissions("MANAGE_GUILD")]
pub fn prefix(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let prefix = args.single::<String>().ok();
    match &prefix {
        Some(prefix) => msg
            .channel_id
            .say(&ctx, format!("Prefix is set to `{}`", prefix))?,
        None => msg.channel_id.say(&ctx, "Prefix is set to `~`")?,
    };
    Settings::set_prefix(ctx, msg.guild_id.expect("guild only"), prefix);
    Ok(())
}

#[command]
#[description("Get link to vote for Modbot on Discord Bot List")]
pub fn vote(ctx: &mut Context, msg: &Message) -> CommandResult {
    let bot = *ctx.cache.read().user.id.as_u64();
    let profile = crate::dbl::get_profile(bot);
    msg.channel_id.say(ctx, format!("{}/vote", profile))?;
    Ok(())
}

#[command]
#[owners_only]
#[help_available(false)]
pub fn servers(ctx: &mut Context, msg: &Message) -> CommandResult {
    use std::fmt::Write;

    let buf = ctx
        .cache
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
    match msg.author.direct_message(&ctx, move |m| m.content(buf)) {
        Ok(_) => {
            if msg.guild_id.is_some() {
                let _ = msg.react(ctx, '\u{01F44C}'); // :ok_hand:
            }
        }
        Err(e) => {
            eprintln!("Error sending server list: {:?}", e);
            let _ = msg
                .channel_id
                .say(ctx, "There was a problem sending you the server list.");
        }
    }
    Ok(())
}
