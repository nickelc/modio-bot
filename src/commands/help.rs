use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::application_command::{
    CommandData, CommandOptionValue,
};
use twilight_model::application::interaction::Interaction;
use twilight_util::builder::command::{CommandBuilder, StringBuilder};
use twilight_util::builder::embed::EmbedBuilder;

use super::create_response;
use crate::bot::Context;
use crate::commands::EphemeralMessage;
use crate::error::Error;

const HELP_ABOUT: (&str, &str) = ("**Command: /about**", include_str!("help/about.md"));
const HELP_GAME: (&str, &str) = ("**Command: /game**", include_str!("help/game.md"));
const HELP_GAMES: (&str, &str) = ("**Command: /games**", include_str!("help/games.md"));
const HELP_MODS: (&str, &str) = ("**Command: /mods**", include_str!("help/mods.md"));
const HELP_POPULAR: (&str, &str) = ("**Command: /popular**", include_str!("help/popular.md"));
const HELP_SETTINGS_DEFAULT_GAME: (&str, &str) = (
    "**Command: /settings default-game**",
    include_str!("help/settings-default-game.md"),
);
const HELP_SUBS_LIST: (&str, &str) = ("**Command: /subs list**", include_str!("help/subs-list.md"));
const HELP_SUBS_ADD: (&str, &str) = ("**Command: /subs add**", include_str!("help/subs-add.md"));
const HELP_SUBS_RM: (&str, &str) = ("**Command: /subs rm**", include_str!("help/subs-rm.md"));
const HELP_SUBS_MODS_MUTED: (&str, &str) = (
    "**Command: /subs mods muted**",
    include_str!("help/subs-mods-muted.md"),
);
const HELP_SUBS_MODS_MUTE: (&str, &str) = (
    "**Command: /subs mods mute**",
    include_str!("help/subs-mods-mute.md"),
);
const HELP_SUBS_MODS_UNMUTE: (&str, &str) = (
    "**Command: /subs mods unmute**",
    include_str!("help/subs-mods-unmute.md"),
);
const HELP_SUBS_USERS_MUTED: (&str, &str) = (
    "**Command: /subs users muted**",
    include_str!("help/subs-users-muted.md"),
);
const HELP_SUBS_USERS_MUTE: (&str, &str) = (
    "**Command: /subs users mute**",
    include_str!("help/subs-users-mute.md"),
);
const HELP_SUBS_USERS_UNMUTE: (&str, &str) = (
    "**Command: /subs users unmute**",
    include_str!("help/subs-users-unmute.md"),
);

pub fn commands() -> Vec<Command> {
    vec![CommandBuilder::new(
        "help",
        "Show help info and commands",
        CommandType::ChatInput,
    )
    .option(
        StringBuilder::new("command", "Command to get help for.")
            .required(true)
            .choices([
                ("about", "about"),
                ("game", "game"),
                ("games", "games"),
                ("mods", "mods"),
                ("popular", "popular"),
                ("settings default-game", "settings default-game"),
                ("subs list", "subs list"),
                ("subs add", "subs add"),
                ("subs rm", "subs rm"),
                ("subs mods muted", "subs mods muted"),
                ("subs mods mute", "subs mods mute"),
                ("subs mods unmute", "subs mods unmute"),
                ("subs users muted", "subs users muted"),
                ("subs users mute", "subs users mute"),
                ("subs users unmute", "subs users unmute"),
            ]),
    )
    .build()]
}

pub async fn help(
    ctx: &Context,
    interaction: &Interaction,
    command: &CommandData,
) -> Result<(), Error> {
    let command = command.options.iter().find_map(|opt| match &opt.value {
        CommandOptionValue::String(value) => Some(value.as_str()),
        _ => None,
    });
    let (title, description) = match command {
        Some("about") => HELP_ABOUT,
        Some("game") => HELP_GAME,
        Some("games") => HELP_GAMES,
        Some("mods") => HELP_MODS,
        Some("popular") => HELP_POPULAR,
        Some("settings default-game") => HELP_SETTINGS_DEFAULT_GAME,
        Some("subs list") => HELP_SUBS_LIST,
        Some("subs add") => HELP_SUBS_ADD,
        Some("subs rm") => HELP_SUBS_RM,
        Some("subs mods muted") => HELP_SUBS_MODS_MUTED,
        Some("subs mods mute") => HELP_SUBS_MODS_MUTE,
        Some("subs mods unmute") => HELP_SUBS_MODS_UNMUTE,
        Some("subs users muted") => HELP_SUBS_USERS_MUTED,
        Some("subs users mute") => HELP_SUBS_USERS_MUTE,
        Some("subs users unmute") => HELP_SUBS_USERS_UNMUTE,
        _ => return Ok(()),
    };
    let data = EmbedBuilder::new()
        .title(title)
        .description(description)
        .into_ephemeral();

    create_response(ctx, interaction, data).await?;
    Ok(())
}
