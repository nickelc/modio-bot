use std::borrow::Cow;

use modio::filter::prelude::*;
use modio::games::ApiAccessOptions;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::application_command::{
    CommandData, CommandDataOption, CommandOptionValue,
};
use twilight_model::application::interaction::Interaction;
use twilight_model::guild::Permissions;
use twilight_util::builder::command::{CommandBuilder, StringBuilder, SubCommandBuilder};
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use super::{create_response, defer_ephemeral, update_response_content};
use crate::bot::Context;
use crate::error::Error;

pub fn commands() -> Vec<Command> {
    vec![
        CommandBuilder::new("about", "Get bot info", CommandType::ChatInput).build(),
        CommandBuilder::new(
            "settings",
            "Guild specific settings",
            CommandType::ChatInput,
        )
        .dm_permission(false)
        .default_member_permissions(Permissions::MANAGE_GUILD)
        .option(
            SubCommandBuilder::new("default-game", "Set the default game for `/mods` command")
                .option(StringBuilder::new("value", "ID or search").required(true)),
        )
        .build(),
    ]
}
pub async fn about(ctx: &Context, interaction: &Interaction) -> Result<(), Error> {
    let user = ctx.cache.current_user().unwrap();

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

    let embed = EmbedBuilder::new()
        .author(EmbedAuthorBuilder::new(user.name))
        .footer(EmbedFooterBuilder::new(format!(
            "Servers: {}",
            ctx.metrics.guilds.get()
        )))
        .field(
            EmbedFieldBuilder::new(
                "Invite to server",
                "[discordbot.mod.io](https://discordbot.mod.io)",
            )
            .inline(),
        )
        .field(
            EmbedFieldBuilder::new("mod.io Discord", "[discord.mod.io](https://discord.mod.io)")
                .inline(),
        )
        .field(
            EmbedFieldBuilder::new(
                "modbot Discord",
                "[discord.gg/XNX9665](https://discord.gg/XNX9665)",
            )
            .inline(),
        )
        .field(
            EmbedFieldBuilder::new(
                "Website/Blog",
                "[ModBot for Discord](https://blog.mod.io/modbot-for-discord-blog-52c07be0d828)",
            )
            .inline(),
        )
        .field(
            EmbedFieldBuilder::new(
                "Github",
                "[nickelc/modio-bot](https://github.com/nickelc/modio-bot)",
            )
            .inline(),
        )
        .field(EmbedFieldBuilder::new("Version", version).inline());

    let data = InteractionResponseDataBuilder::new().embeds([embed.build()]);

    create_response(ctx, interaction, data.build()).await?;

    Ok(())
}

pub async fn settings(
    ctx: &Context,
    interaction: &Interaction,
    command: &CommandData,
) -> Result<(), Error> {
    let filter = match command.options.as_slice() {
        [CommandDataOption {
            value: CommandOptionValue::SubCommand(commands),
            ..
        }] => match commands.as_slice() {
            [CommandDataOption {
                value: CommandOptionValue::String(s),
                ..
            }] => match s.parse::<u32>() {
                Ok(id) => Id::eq(id),
                Err(_) => s
                    .strip_prefix('@')
                    .map_or_else(|| Fulltext::eq(s), NameId::eq),
            },
            _ => unreachable!(),
        },

        _ => unreachable!(),
    };

    defer_ephemeral(ctx, interaction).await?;

    let game = ctx.modio.games().search(filter).first().await?;
    let guild_id = interaction.guild_id.expect("guild only command");

    let content: Cow<'_, str> = if let Some(game) = game {
        if game
            .api_access_options
            .contains(ApiAccessOptions::ALLOW_THIRD_PARTY)
        {
            let mut settings = ctx.settings.lock().unwrap();
            settings.set_game(guild_id.get(), game.id)?;
            format!("Game is set to '{}'.", game.name).into()
        } else {
            let msg = format!(
                ":no_entry: Third party API access is disabled for '{}' but is required for the commands.",
                game.name
            );
            msg.into()
        }
    } else {
        "Game not found.".into()
    };

    update_response_content(ctx, interaction, &content).await
}
