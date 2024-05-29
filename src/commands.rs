use modio::games::Game;
use twilight_http::client::InteractionClient;
use twilight_model::application::command::{
    Command, CommandOptionChoice, CommandOptionChoiceValue,
};
use twilight_model::application::interaction::application_command::{
    CommandData, CommandDataOption, CommandOptionValue,
};
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::MessageFlags;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::embed::EmbedBuilder;
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::bot::Context;
use crate::db::autocomplete::{games_by_name, games_by_name_id};
use crate::db::types::{ChannelId, GuildId};
use crate::error::Error;

mod basic;
mod game;
mod help;
pub mod mods;
mod subs;

fn commands() -> Vec<Command> {
    let mut cmds = Vec::new();
    cmds.extend(help::commands());
    cmds.extend(basic::commands());
    cmds.extend(game::commands());
    cmds.extend(mods::commands());
    cmds.extend(subs::commands());
    cmds
}

pub async fn register(client: &InteractionClient<'_>) -> Result<(), Error> {
    client.set_global_commands(&commands()).await?;
    Ok(())
}

pub async fn handle_command(ctx: &Context, interaction: &Interaction, command: &CommandData) {
    ctx.metrics.commands.inc(&command.name);

    let res = match command.name.as_str() {
        "about" => basic::about(ctx, interaction).await,
        "help" => help::help(ctx, interaction, command).await,
        "settings" => basic::settings(ctx, interaction, command).await,
        "games" => game::games(ctx, interaction, command).await,
        "game" => game::game(ctx, interaction).await,
        "mods" => mods::list(ctx, interaction, command).await,
        "popular" => mods::popular(ctx, interaction, command).await,
        "subs" => subs::handle_command(ctx, interaction, command).await,
        _ => Ok(()),
    };
    if let Err(e) = res {
        tracing::error!("{e}");
    }
}

pub async fn handle_component(
    ctx: &Context,
    interaction: &Interaction,
    component: &MessageComponentInteractionData,
) {
    let res = match interaction
        .message
        .as_ref()
        .and_then(|m| m.interaction.as_ref())
    {
        Some(msg) if msg.name == "mods" => mods::list_component(ctx, interaction, component).await,
        _ => Ok(()),
    };
    if let Err(e) = res {
        tracing::error!("{e}");
    }
}

trait EphemeralMessage {
    fn into_ephemeral(self) -> InteractionResponseData;
}

impl EphemeralMessage for &str {
    fn into_ephemeral(self) -> InteractionResponseData {
        InteractionResponseDataBuilder::new()
            .ephemeral(self)
            .build()
    }
}

impl EphemeralMessage for String {
    fn into_ephemeral(self) -> InteractionResponseData {
        InteractionResponseDataBuilder::new()
            .ephemeral(self)
            .build()
    }
}

impl EphemeralMessage for EmbedBuilder {
    fn into_ephemeral(self) -> InteractionResponseData {
        let embed = self.build();
        InteractionResponseDataBuilder::new()
            .flags(MessageFlags::EPHEMERAL)
            .embeds([embed])
            .build()
    }
}

trait InteractionExt {
    fn guild_id(&self) -> Option<GuildId>;
    fn channel_id(&self) -> Option<ChannelId>;
}

impl InteractionExt for Interaction {
    fn guild_id(&self) -> Option<GuildId> {
        self.guild_id.map(GuildId)
    }

    fn channel_id(&self) -> Option<ChannelId> {
        self.channel.as_ref().map(|c| ChannelId(c.id))
    }
}

trait InteractionResponseBuilderExt {
    fn ephemeral(self, content: impl Into<String>) -> InteractionResponseDataBuilder;
}

impl InteractionResponseBuilderExt for InteractionResponseDataBuilder {
    fn ephemeral(self, content: impl Into<String>) -> InteractionResponseDataBuilder {
        self.content(content).flags(MessageFlags::EPHEMERAL)
    }
}

trait SubCommandExt {
    fn subcommand(&self) -> Option<(&str, &[CommandDataOption])>;
}

fn find_subcommand(opts: &[CommandDataOption]) -> Option<(&str, &[CommandDataOption])> {
    opts.iter().find_map(|opt| match &opt.value {
        CommandOptionValue::SubCommandGroup(opts) | CommandOptionValue::SubCommand(opts) => {
            Some((opt.name.as_str(), opts.as_slice()))
        }
        _ => None,
    })
}

impl SubCommandExt for &CommandData {
    fn subcommand(&self) -> Option<(&str, &[CommandDataOption])> {
        find_subcommand(&self.options)
    }
}

impl SubCommandExt for &[CommandDataOption] {
    fn subcommand(&self) -> Option<(&str, &[CommandDataOption])> {
        find_subcommand(self)
    }
}

trait AutocompleteExt {
    fn autocomplete(&self) -> Option<(&str, &str)>;
}

fn find_autocomplete_option(opts: &[CommandDataOption]) -> Option<(&str, &str)> {
    for opt in opts {
        match &opt.value {
            CommandOptionValue::SubCommand(opts) | CommandOptionValue::SubCommandGroup(opts) => {
                return find_autocomplete_option(opts)
            }
            CommandOptionValue::Focused(value, _) => return Some((&opt.name, value)),
            _ => {}
        }
    }
    None
}

impl AutocompleteExt for &CommandData {
    fn autocomplete(&self) -> Option<(&str, &str)> {
        find_autocomplete_option(&self.options)
    }
}

async fn create_response(
    ctx: &Context,
    interaction: &Interaction,
    data: InteractionResponseData,
) -> Result<(), Error> {
    let response = InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(data),
    };
    ctx.interaction()
        .create_response(interaction.id, &interaction.token, &response)
        .await?;
    Ok(())
}

async fn defer_ephemeral(ctx: &Context, interaction: &Interaction) -> Result<(), Error> {
    ctx.interaction()
        .create_response(
            interaction.id,
            &interaction.token,
            &InteractionResponse {
                kind: InteractionResponseType::DeferredChannelMessageWithSource,
                data: Some(
                    InteractionResponseDataBuilder::new()
                        .flags(MessageFlags::EPHEMERAL)
                        .build(),
                ),
            },
        )
        .await?;
    Ok(())
}

async fn defer_response(ctx: &Context, interaction: &Interaction) -> Result<(), Error> {
    ctx.interaction()
        .create_response(
            interaction.id,
            &interaction.token,
            &InteractionResponse {
                kind: InteractionResponseType::DeferredChannelMessageWithSource,
                data: None,
            },
        )
        .await?;
    Ok(())
}

async fn defer_component_response(ctx: &Context, interaction: &Interaction) -> Result<(), Error> {
    ctx.interaction()
        .create_response(
            interaction.id,
            &interaction.token,
            &InteractionResponse {
                kind: InteractionResponseType::DeferredUpdateMessage,
                data: None,
            },
        )
        .await?;
    Ok(())
}

async fn update_response_content(
    ctx: &Context,
    interaction: &Interaction,
    content: &str,
) -> Result<(), Error> {
    ctx.interaction()
        .update_response(&interaction.token)
        .content(Some(content))?
        .await?;
    Ok(())
}

async fn update_response_from_content(
    ctx: &Context,
    interaction: &Interaction,
    title: &str,
    contents: &[String],
) -> Result<(), Error> {
    let mut contents = contents.iter();
    if let Some(content) = contents.next() {
        let embed = EmbedBuilder::new()
            .title(title)
            .description(content)
            .build();

        ctx.interaction()
            .update_response(&interaction.token)
            .embeds(Some(&[embed]))?
            .await?;

        for content in contents {
            let embed = EmbedBuilder::new()
                .title(title)
                .description(content)
                .build();

            ctx.interaction()
                .create_followup(&interaction.token)
                .embeds(&[embed])?
                .await?;
        }
    }
    Ok(())
}

async fn autocomplete_games(
    ctx: &Context,
    interaction: &Interaction,
    value: &str,
) -> Result<(), Error> {
    let games = value.strip_prefix('@').map_or_else(
        || games_by_name(&ctx.pool, value),
        |value| games_by_name_id(&ctx.pool, value),
    )?;

    let choices = games.into_iter().map(|(id, name)| CommandOptionChoice {
        name,
        name_localizations: None,
        value: CommandOptionChoiceValue::String(id.to_string()),
    });
    let data = InteractionResponseDataBuilder::new()
        .choices(choices)
        .build();
    let response = InteractionResponse {
        kind: InteractionResponseType::ApplicationCommandAutocompleteResult,
        data: Some(data),
    };
    ctx.interaction()
        .create_response(interaction.id, &interaction.token, &response)
        .await?;
    Ok(())
}

async fn search_game(ctx: &Context, search: &str) -> Result<Option<Game>, Error> {
    use crate::util::IntoFilter;

    let filter = search.into_filter();
    let game = ctx.modio.games().search(filter).first().await?;
    Ok(game)
}
