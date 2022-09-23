use std::fmt::Write;

use futures_util::{future, TryStreamExt};
use modio::filter::prelude::*;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::application_command::{
    CommandData, CommandDataOption, CommandOptionValue,
};
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::embed::EmbedField;
use twilight_util::builder::command::{CommandBuilder, StringBuilder};
use twilight_util::builder::embed::{EmbedBuilder, ImageSource};
use twilight_util::builder::InteractionResponseDataBuilder;

use super::{create_response, create_responses_from_content, InteractionResponseBuilderExt};
use crate::bot::Context;
use crate::error::Error;
use crate::util::ContentBuilder;

pub fn commands() -> Vec<Command> {
    vec![
        CommandBuilder::new(
            "games",
            "List all games on <https://mod.io>",
            CommandType::ChatInput,
        )
        .option(StringBuilder::new("search", "ID or search"))
        .build(),
        CommandBuilder::new("game", "Display the default game.", CommandType::ChatInput)
            .dm_permission(false)
            .build(),
    ]
}

pub async fn games(
    ctx: &Context,
    interaction: &Interaction,
    command: &CommandData,
) -> Result<(), Error> {
    let filter = match command.options.as_slice() {
        [CommandDataOption {
            value: CommandOptionValue::String(s),
            ..
        }] => match s.parse::<u32>() {
            Ok(id) => Id::eq(id),
            Err(_) => Fulltext::eq(s),
        },
        _ => Filter::default(),
    };

    let games = ctx
        .modio
        .games()
        .search(filter)
        .iter()
        .await?
        .try_fold(ContentBuilder::new(4000), |mut buf, game| {
            let _ = writeln!(&mut buf, "{}. {}", game.id, game.name);
            async { Ok(buf) }
        })
        .await?;

    create_responses_from_content(ctx, interaction, "Games", &games.content).await
}

pub async fn game(ctx: &Context, interaction: &Interaction) -> Result<(), Error> {
    let game_id = {
        let settings = ctx.settings.lock().unwrap();
        interaction.guild_id.and_then(|id| settings.game(id.get()))
    };

    let mut builder = InteractionResponseDataBuilder::new();
    if let Some(id) = game_id {
        let stats = ctx.modio.game(id).statistics();
        let (game, stats) = future::try_join(ctx.modio.game(id).get(), stats).await?;

        let embed = EmbedBuilder::new()
            .title(game.name)
            .url(game.profile_url.to_string())
            .description(game.summary)
            .image(ImageSource::url(game.logo.thumb_640x360).unwrap())
            .field(EmbedField {
                name: "Info".into(),
                value: format!(
                    r#"**Id:** {}
**Name-Id:** {}
**Profile:** {}"#,
                    game.id, game.name_id, game.profile_url,
                ),
                inline: true,
            })
            .field(EmbedField {
                name: "Stats".into(),
                value: format!(
                    r#"**Mods:** {}
**Subscribers:** {}
**Downloads:** {}"#,
                    stats.mods_total, stats.subscribers_total, stats.downloads.total,
                ),
                inline: true,
            })
            .build();

        builder = builder.embeds([embed]);
    } else {
        builder = builder.ephemeral("Default game is not set.");
    }

    create_response(ctx, interaction, builder.build()).await?;

    Ok(())
}
