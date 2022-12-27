use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write;

use futures_util::stream::FuturesUnordered;
use futures_util::{future, TryStreamExt};
use modio::filter::prelude::*;
use modio::games::{ApiAccessOptions, Game};
use modio::mods::Mod;
use modio::Modio;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::application_command::{
    CommandData, CommandDataOption, CommandOptionValue,
};
use twilight_model::application::interaction::Interaction;
use twilight_model::guild::Permissions;
use twilight_util::builder::command::{
    CommandBuilder, IntegerBuilder, StringBuilder, SubCommandBuilder, SubCommandGroupBuilder,
};
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder};

use super::{
    autocomplete_games, create_response, defer_ephemeral, search_game, update_response_content,
    update_response_from_content, EphemeralMessage,
};
use crate::bot::Context;
use crate::db::types::{ChannelId, GameId, GuildId, ModId};
use crate::db::{Events, Tags};
use crate::error::Error;
use crate::util::{ContentBuilder, IntoFilter};

pub fn commands() -> Vec<Command> {
    vec![CommandBuilder::new(
        "subs",
        "Manage subscriptions in the current channel to mod updates of a game.",
        CommandType::ChatInput,
    )
    .option(SubCommandBuilder::new(
        "overview",
        "Show an overview of the current setup of this server.",
    ))
    .option(SubCommandBuilder::new("list", "List subscriptions"))
    .option(
        SubCommandBuilder::new(
            "add",
            "Subscribe the current channel to mod update of a game.",
        )
        .option(
            StringBuilder::new("game", "ID or search")
                .required(true)
                .autocomplete(true),
        )
        .option(StringBuilder::new("tags", "Comma-separated list of tags"))
        .option(
            IntegerBuilder::new("type", "Type of the mod updates.").choices([
                ("New mods", i64::from(Events::NEW.bits())),
                ("Updated mods", i64::from(Events::UPD.bits())),
                ("All", i64::from(Events::ALL.bits())),
            ]),
        ),
    )
    .option(
        SubCommandBuilder::new(
            "rm",
            "Unsubscribe the current channel from mod update of a game.",
        )
        .option(
            StringBuilder::new("game", "ID or search")
                .required(true)
                .autocomplete(true),
        )
        .option(StringBuilder::new("tags", "Comma-separated list of tags"))
        .option(
            IntegerBuilder::new("type", "Type of the mod updates.").choices([
                ("New mods", i64::from(Events::NEW.bits())),
                ("Updated mods", i64::from(Events::UPD.bits())),
                ("All", i64::from(Events::ALL.bits())),
            ]),
        ),
    )
    .option(
        SubCommandGroupBuilder::new("mods", "Mute update notifications for a mod.").subcommands([
            SubCommandBuilder::new("muted", "List muted mods"),
            SubCommandBuilder::new("mute", "Mute update notifications for a mod.")
                .option(
                    StringBuilder::new("game", "ID or search")
                        .required(true)
                        .autocomplete(true),
                )
                .option(StringBuilder::new("mod", "ID or search").required(true)),
            SubCommandBuilder::new("unmute", "Unmute update notifications for a mod.")
                .option(
                    StringBuilder::new("game", "ID or search")
                        .required(true)
                        .autocomplete(true),
                )
                .option(StringBuilder::new("mod", "ID or search").required(true)),
        ]),
    )
    .option(
        SubCommandGroupBuilder::new("users", "Mute update notifications for mods of a user.")
            .subcommands([
                SubCommandBuilder::new("muted", "List muted user"),
                SubCommandBuilder::new("mute", "Mute update notifications for mods of a user.")
                    .option(
                        StringBuilder::new("game", "ID or search")
                            .required(true)
                            .autocomplete(true),
                    )
                    .option(StringBuilder::new("name", "username").required(true)),
                SubCommandBuilder::new("unmute", "Unmute update notifications for mods of a user.")
                    .option(
                        StringBuilder::new("game", "ID or search")
                            .required(true)
                            .autocomplete(true),
                    )
                    .option(StringBuilder::new("name", "username").required(true)),
            ]),
    )
    .dm_permission(false)
    .default_member_permissions(Permissions::MANAGE_CHANNELS)
    .build()]
}

pub async fn handle_command(
    ctx: &Context,
    interaction: &Interaction,
    command: &CommandData,
) -> Result<(), Error> {
    let subcommand = command.options.iter().find_map(|e| match &e.value {
        CommandOptionValue::SubCommand(opts) | CommandOptionValue::SubCommandGroup(opts) => {
            Some((e.name.as_str(), opts))
        }
        _ => None,
    });

    match subcommand {
        Some(("overview", _)) => overview(ctx, interaction).await,
        Some(("list", _)) => list(ctx, interaction).await,
        Some(("add", opts)) => subscribe(ctx, interaction, opts).await,
        Some(("rm", opts)) => unsubscribe(ctx, interaction, opts).await,
        Some(("mods", opts)) => mods(ctx, interaction, opts).await,
        Some(("users", opts)) => users(ctx, interaction, opts).await,
        _ => Ok(()),
    }
}

async fn overview(ctx: &Context, interaction: &Interaction) -> Result<(), Error> {
    let guild_id = interaction.guild_id.map(GuildId).unwrap();

    let (subs, excluded_mods, excluded_users) = ctx.subscriptions.list_for_overview(guild_id)?;

    if subs.is_empty() && excluded_mods.is_empty() && excluded_users.is_empty() {
        let data = "No subscriptions found.".into_ephemeral();
        return create_response(ctx, interaction, data).await;
    }

    defer_ephemeral(ctx, interaction).await?;

    // Collect all game ids to fetch from modio.
    let mut game_ids = subs
        .values()
        .flatten()
        .map(|(g, _, _)| g)
        .chain(excluded_mods.keys().map(|(g, _)| g))
        .chain(excluded_users.keys().map(|(g, _)| g))
        .collect::<Vec<_>>();

    game_ids.sort_unstable();
    game_ids.dedup();

    let filter = Id::_in(game_ids);
    let list = ctx.modio.games().search(filter).collect().await?;
    let games = list
        .into_iter()
        .map(|g| (g.id, g.name))
        .collect::<HashMap<_, _>>();

    let mut embed = EmbedBuilder::new().title("Subscriptions");

    let mut content = String::new();
    for (channel_id, subs) in subs {
        let _ = writeln!(&mut content, "__Channel:__ <#{channel_id}>");
        for (game_id, tags, evts) in subs {
            if let Some(game) = games.get(&game_id) {
                let _ = write!(&mut content, "{game_id}. {game}");
            } else {
                let _ = write!(&mut content, "{game_id}");
            }
            content.push_str(subs_suffix(evts));

            if !tags.is_empty() {
                content.push_str(" | Tags: ");
                push_tags(&mut content, tags.iter());
            }
            content.push('\n');
        }
        content.push('\n');
    }
    embed = embed.description(content);

    if !excluded_mods.is_empty() {
        let mut content = String::new();
        for ((game_id, channel_id), mods) in excluded_mods {
            let _ = writeln!(&mut content, "__Channel:__ <#{channel_id}>");
            if let Some(game) = games.get(&game_id) {
                let _ = write!(&mut content, "*{game_id}. {game}:* ");
            } else {
                let _ = write!(&mut content, "*{game_id}:* ");
            }
            let mut it = mods.iter().peekable();
            while let Some(mod_) = it.next() {
                let _ = write!(&mut content, "{mod_}");
                if it.peek().is_some() {
                    content.push_str(", ");
                }
            }
            content.push_str("\n\n");
        }
        embed = embed.field(EmbedFieldBuilder::new("Muted mods", content));
    }

    if !excluded_users.is_empty() {
        let mut content = String::new();
        for ((game_id, channel_id), users) in excluded_users {
            let _ = writeln!(&mut content, "__Channel:__ <#{channel_id}>");
            if let Some(game) = games.get(&game_id) {
                let _ = write!(&mut content, "*{game_id}. {game}:* ");
            } else {
                let _ = write!(&mut content, "*{game_id}:* ");
            }
            let mut it = users.iter().peekable();
            while let Some(user) = it.next() {
                let _ = write!(&mut content, "`{user}`");
                if it.peek().is_some() {
                    content.push_str(", ");
                }
            }
            content.push_str("\n\n");
        }
        embed = embed.field(EmbedFieldBuilder::new("Muted users", content));
    }
    ctx.interaction()
        .update_response(&interaction.token)
        .embeds(Some(&[embed.build()]))?
        .await?;

    Ok(())
}

async fn list(ctx: &Context, interaction: &Interaction) -> Result<(), Error> {
    let channel_id = interaction.channel_id.map(ChannelId).unwrap();
    let subs = ctx.subscriptions.list_for_channel(channel_id)?;

    if subs.is_empty() {
        let data = "No subscriptions found.".into_ephemeral();
        return create_response(ctx, interaction, data).await;
    }

    defer_ephemeral(ctx, interaction).await?;

    let filter = Id::_in(subs.iter().map(|s| s.0).collect::<Vec<_>>());
    let list = ctx.modio.games().search(filter).collect().await?;
    let games = list
        .into_iter()
        .map(|g| (g.id, g.name))
        .collect::<HashMap<_, _>>();

    let mut content = String::new();
    for (game_id, tags, evts) in subs {
        let Some(name) = games.get(&game_id) else {
            continue;
        };
        let _ = write!(&mut content, "{game_id}. {name}");

        content.push_str(subs_suffix(evts));

        if !tags.is_empty() {
            content.push_str(" | Tags: ");
            push_tags(&mut content, tags.iter());
        }
        content.push('\n');
    }

    let embed = EmbedBuilder::new()
        .title("Subscriptions")
        .description(content)
        .build();

    ctx.interaction()
        .update_response(&interaction.token)
        .embeds(Some(&[embed]))?
        .await?;

    Ok(())
}

async fn subscribe(
    ctx: &Context,
    interaction: &Interaction,
    opts: &[CommandDataOption],
) -> Result<(), Error> {
    for opt in opts {
        match &opt.value {
            CommandOptionValue::Focused(value, _) if opt.name == "game" => {
                return autocomplete_games(ctx, interaction, value).await;
            }
            _ => {}
        }
    }

    let mut game = None;
    let mut tags = None;
    let mut evts = Events::ALL;

    defer_ephemeral(ctx, interaction).await?;

    for opt in opts {
        match &opt.value {
            CommandOptionValue::String(s) if opt.name == "game" => {
                game = search_game(ctx, s).await?;

                if game.is_none() {
                    let content = "Game not found.";
                    return update_response_content(ctx, interaction, content).await;
                }
            }
            CommandOptionValue::String(s) if opt.name == "tags" => {
                tags = Some(s.as_str());
            }
            CommandOptionValue::Integer(v) if opt.name == "type" => {
                evts = if (1..=3).contains(v) {
                    #[allow(clippy::cast_possible_truncation)]
                    Events::from_bits_truncate(*v as i32)
                } else {
                    Events::ALL
                };
            }
            _ => {}
        }
    }

    let game = game.expect("required option");
    if !game
        .api_access_options
        .contains(ApiAccessOptions::ALLOW_THIRD_PARTY)
    {
        let content = format!(
            ":no_entry: Third party API access is disabled for '{}' but is required for subscriptions.",
            game.name
        );
        return update_response_content(ctx, interaction, &content).await;
    }

    let channel_id = interaction.channel_id.map(ChannelId).unwrap();
    let guild_id = interaction.guild_id.map(GuildId).unwrap();

    let game_tags = game
        .tag_options
        .into_iter()
        .flat_map(|opt| opt.tags)
        .collect::<Tags>();

    let (hidden, mut sub_tags) = tags
        .map(|s| Tags::from_csv(s).partition())
        .unwrap_or_default();

    if !sub_tags.is_subset(&game_tags) {
        let mut content = format!("Failed to subscribe to '{}'.\n", game.name);
        content.push_str("Invalid tag(s): ");
        push_tags(&mut content, sub_tags.difference(&game_tags));

        content.push_str("\nAvailable tags: ");
        push_tags(&mut content, game_tags.iter());

        return update_response_content(ctx, interaction, &content).await;
    }
    sub_tags.extend(hidden);

    let ret = ctx
        .subscriptions
        .add(GameId(game.id), channel_id, sub_tags, guild_id, evts);

    let content: Cow<'_, str> = match ret {
        Ok(_) => format!("Subscribed to '{}'.", game.name).into(),
        Err(e) => {
            tracing::error!("{e}");

            "Failed to add subscription.".into()
        }
    };

    update_response_content(ctx, interaction, &content).await
}

async fn unsubscribe(
    ctx: &Context,
    interaction: &Interaction,
    opts: &[CommandDataOption],
) -> Result<(), Error> {
    for opt in opts {
        match &opt.value {
            CommandOptionValue::Focused(value, _) if opt.name == "game" => {
                return autocomplete_games(ctx, interaction, value).await;
            }
            _ => {}
        }
    }

    let mut game = None;
    let mut tags = None;
    let mut evts = Events::ALL;

    defer_ephemeral(ctx, interaction).await?;

    for opt in opts {
        match &opt.value {
            CommandOptionValue::String(s) if opt.name == "game" => {
                game = search_game(ctx, s).await?;

                if game.is_none() {
                    let content = "Game not found.";
                    return update_response_content(ctx, interaction, content).await;
                }
            }
            CommandOptionValue::String(s) if opt.name == "tags" => {
                tags = Some(s.as_str());
            }
            CommandOptionValue::Integer(v) if opt.name == "type" => {
                evts = if (1..=3).contains(v) {
                    #[allow(clippy::cast_possible_truncation)]
                    Events::from_bits_truncate(*v as i32)
                } else {
                    Events::ALL
                };
            }
            _ => {}
        }
    }

    let game = game.expect("required option");
    let channel_id = interaction.channel_id.map(ChannelId).unwrap();

    let game_tags = game
        .tag_options
        .into_iter()
        .flat_map(|opt| opt.tags)
        .collect::<Tags>();

    let (hidden, mut sub_tags) = tags
        .map(|s| Tags::from_csv(s).partition())
        .unwrap_or_default();

    if !sub_tags.is_subset(&game_tags) {
        let mut content = format!("Failed to unsubscribe from '{}'.\n", game.name);
        content.push_str("Invalid tag(s): ");
        push_tags(&mut content, sub_tags.difference(&game_tags));

        content.push_str("\nAvailable tags: ");
        push_tags(&mut content, game_tags.iter());

        return update_response_content(ctx, interaction, &content).await;
    }
    sub_tags.extend(hidden);

    let ret = ctx
        .subscriptions
        .remove(GameId(game.id), channel_id, sub_tags, evts);

    let content: Cow<'_, str> = match ret {
        Ok(_) => format!("Unsubscribed from '{}'.", game.name).into(),
        Err(e) => {
            tracing::error!("{e}");

            "Failed to remove subscription.".into()
        }
    };

    update_response_content(ctx, interaction, &content).await
}

/// `/subs mods`
async fn mods(
    ctx: &Context,
    interaction: &Interaction,
    opts: &[CommandDataOption],
) -> Result<(), Error> {
    let subcommand = opts.iter().find_map(|e| match &e.value {
        CommandOptionValue::SubCommand(opts) => Some((e.name.as_str(), opts)),
        _ => None,
    });

    if let Some((_, opts)) = subcommand {
        for opt in opts {
            match &opt.value {
                CommandOptionValue::Focused(value, _) if opt.name == "game" => {
                    return autocomplete_games(ctx, interaction, value).await;
                }
                _ => {}
            }
        }
    }

    defer_ephemeral(ctx, interaction).await?;

    match subcommand {
        Some(("muted", opts)) => mods_muted(ctx, interaction, opts).await,
        Some(("mute", opts)) => mods_mute(ctx, interaction, opts).await,
        Some(("unmute", opts)) => mods_unmute(ctx, interaction, opts).await,
        _ => Ok(()),
    }
}

/// `/subs mods muted`
async fn mods_muted(
    ctx: &Context,
    interaction: &Interaction,
    _opts: &[CommandDataOption],
) -> Result<(), Error> {
    let channel_id = interaction.channel_id.map(ChannelId).unwrap();
    let excluded = ctx.subscriptions.list_excluded_mods(channel_id)?;

    let muted = match excluded.len() {
        0 => {
            let content = "No mod is muted.";
            return update_response_content(ctx, interaction, content).await;
        }
        1 => {
            let (GameId(game), mods) = excluded.into_iter().next().unwrap();
            let filter = Id::_in(mods.into_iter().collect::<Vec<_>>());
            ctx.modio
                .game(game)
                .mods()
                .search(filter)
                .iter()
                .await?
                .try_fold(ContentBuilder::new(4000), |mut buf, m| {
                    let _ = writeln!(&mut buf, "{}. {}", m.id, m.name);
                    async { Ok(buf) }
                })
                .await?
        }
        _ => {
            excluded
                .into_iter()
                .map(|(GameId(game), mods)| {
                    let filter = Id::_in(mods.into_iter().collect::<Vec<_>>());
                    future::try_join(
                        ctx.modio.game(game).get(),
                        ctx.modio.game(game).mods().search(filter).collect(),
                    )
                })
                .collect::<FuturesUnordered<_>>()
                .try_fold(ContentBuilder::new(4000), |mut buf, (game, mods)| {
                    let _ = writeln!(&mut buf, "**{}**", game.name);
                    for m in mods {
                        let _ = writeln!(&mut buf, "{}. {}", m.id, m.name);
                    }
                    let _ = writeln!(&mut buf);
                    async { Ok(buf) }
                })
                .await?
        }
    };

    update_response_from_content(ctx, interaction, "Muted Mods", &muted.content).await
}

/// `/subs mods mute <game> <mod>`
async fn mods_mute(
    ctx: &Context,
    interaction: &Interaction,
    opts: &[CommandDataOption],
) -> Result<(), Error> {
    let mut game_filter = None;
    let mut mod_filter = None;
    for opt in opts {
        match &opt.value {
            CommandOptionValue::String(s) if opt.name == "game" => {
                game_filter = Some(s.into_filter());
            }
            CommandOptionValue::String(s) if opt.name == "mod" => {
                mod_filter = Some(s.into_filter());
            }
            _ => {}
        }
    }

    let game_filter = game_filter.expect("required option");
    let mod_filter = mod_filter.expect("required option");

    let game_mod = find_game_mod(ctx.modio.clone(), game_filter, mod_filter).await?;

    let content: Cow<'_, str> = match game_mod {
        (None, _) => "Game not found.".into(),
        (_, None) => "Mod not found.".into(),
        (Some(game), Some(mod_)) => {
            let channel_id = interaction.channel_id.map(ChannelId).unwrap();
            let guild_id = interaction.guild_id.map(GuildId).unwrap();

            let ret =
                ctx.subscriptions
                    .mute_mod(GameId(game.id), channel_id, guild_id, ModId(mod_.id));

            let content = if let Err(e) = ret {
                tracing::error!("{e}");

                format!("Failed to mute '{}'.", mod_.name)
            } else {
                format!("The mod '{}' is now muted.", mod_.name)
            };

            content.into()
        }
    };

    update_response_content(ctx, interaction, &content).await
}

/// `/subs mods unmute <game> <mod>`
async fn mods_unmute(
    ctx: &Context,
    interaction: &Interaction,
    opts: &[CommandDataOption],
) -> Result<(), Error> {
    let mut game_filter = None;
    let mut mod_filter = None;
    for opt in opts {
        match &opt.value {
            CommandOptionValue::String(s) if opt.name == "game" => {
                game_filter = Some(s.into_filter());
            }
            CommandOptionValue::String(s) if opt.name == "mod" => {
                mod_filter = Some(s.into_filter());
            }
            _ => {}
        }
    }

    let game_filter = game_filter.expect("required option");
    let mod_filter = mod_filter.expect("required option");

    let game_mod = find_game_mod(ctx.modio.clone(), game_filter, mod_filter).await?;

    let content: Cow<'_, str> = match game_mod {
        (None, _) => "Game not found.".into(),
        (_, None) => "Mod not found.".into(),
        (Some(game), Some(mod_)) => {
            let channel_id = interaction.channel_id.map(ChannelId).unwrap();

            let ret = ctx
                .subscriptions
                .unmute_mod(GameId(game.id), channel_id, ModId(mod_.id));

            let content = if let Err(e) = ret {
                tracing::error!("{e}");

                format!("Failed to unmute '{}'.", mod_.name)
            } else {
                format!("The mod '{}' is now unmuted.", mod_.name)
            };

            content.into()
        }
    };

    update_response_content(ctx, interaction, &content).await
}

/// `/subs users`
async fn users(
    ctx: &Context,
    interaction: &Interaction,
    opts: &[CommandDataOption],
) -> Result<(), Error> {
    let subcommand = opts.iter().find_map(|e| match &e.value {
        CommandOptionValue::SubCommand(opts) => Some((e.name.as_str(), opts)),
        _ => None,
    });

    if let Some((_, opts)) = subcommand {
        for opt in opts {
            match &opt.value {
                CommandOptionValue::Focused(value, _) if opt.name == "game" => {
                    return autocomplete_games(ctx, interaction, value).await;
                }
                _ => {}
            }
        }
    }

    defer_ephemeral(ctx, interaction).await?;

    match subcommand {
        Some(("muted", opts)) => users_muted(ctx, interaction, opts).await,
        Some(("mute", opts)) => users_mute(ctx, interaction, opts).await,
        Some(("unmute", opts)) => users_unmute(ctx, interaction, opts).await,
        _ => Ok(()),
    }
}

/// `/subs users muted`
async fn users_muted(
    ctx: &Context,
    interaction: &Interaction,
    _opts: &[CommandDataOption],
) -> Result<(), Error> {
    let channel_id = interaction.channel_id.map(ChannelId).unwrap();
    let excluded = ctx.subscriptions.list_excluded_users(channel_id)?;

    let muted = match excluded.len() {
        0 => {
            let content = "No user is muted.";
            return update_response_content(ctx, interaction, content).await;
        }
        1 => {
            let (_, users) = excluded.into_iter().next().unwrap();

            let mut muted = ContentBuilder::new(4000);
            for (i, name) in users.iter().enumerate() {
                let _ = writeln!(&mut muted, "{}. {name}", i + 1);
            }
            muted
        }
        _ => {
            excluded
                .into_iter()
                .map(|(GameId(game), users)| {
                    future::try_join(ctx.modio.game(game).get(), async { Ok(users) })
                })
                .collect::<FuturesUnordered<_>>()
                .try_fold(ContentBuilder::new(4000), |mut buf, (game, users)| {
                    let _ = writeln!(&mut buf, "**{}**", game.name);
                    for (i, name) in users.iter().enumerate() {
                        let _ = writeln!(&mut buf, "{}. {name}", i + 1);
                    }
                    let _ = writeln!(&mut buf);
                    async { Ok(buf) }
                })
                .await?
        }
    };

    update_response_from_content(ctx, interaction, "Muted Users", &muted.content).await
}

/// `/subs users mute <game> <username>`
async fn users_mute(
    ctx: &Context,
    interaction: &Interaction,
    opts: &[CommandDataOption],
) -> Result<(), Error> {
    let mut game_filter = None;
    let mut name = None;
    for opt in opts {
        match &opt.value {
            CommandOptionValue::String(s) if opt.name == "game" => {
                game_filter = Some(s.into_filter());
            }
            CommandOptionValue::String(s) if opt.name == "name" => {
                name = Some(s);
            }
            _ => {}
        }
    }

    let game_filter = game_filter.expect("required option");
    let name = name.expect("required option");

    let game = ctx.modio.games().search(game_filter).first().await?;
    let content: Cow<'_, str> = match game {
        Some(game) => {
            let guild_id = interaction.guild_id.map(GuildId).unwrap();
            let channel_id = interaction.channel_id.map(ChannelId).unwrap();

            let ret = ctx
                .subscriptions
                .mute_user(GameId(game.id), channel_id, guild_id, name);

            let content = if let Err(e) = ret {
                tracing::error!("{e}");

                format!("Failed to mute '{name}'.")
            } else {
                format!("The user '{name}' is now muted for '{}'.", game.name)
            };

            content.into()
        }
        None => "Game not found.".into(),
    };

    update_response_content(ctx, interaction, &content).await
}

/// `/subs users unmute <game> <username>`
async fn users_unmute(
    ctx: &Context,
    interaction: &Interaction,
    opts: &[CommandDataOption],
) -> Result<(), Error> {
    let mut game_filter = None;
    let mut name = None;
    for opt in opts {
        match &opt.value {
            CommandOptionValue::String(s) if opt.name == "game" => {
                game_filter = Some(s.into_filter());
            }
            CommandOptionValue::String(s) if opt.name == "name" => {
                name = Some(s);
            }
            _ => {}
        }
    }

    let game_filter = game_filter.expect("required option");
    let name = name.expect("required option");

    let game = ctx.modio.games().search(game_filter).first().await?;
    let content: Cow<'_, str> = match game {
        Some(game) => {
            let channel_id = interaction.channel_id.map(ChannelId).unwrap();

            let ret = ctx
                .subscriptions
                .unmute_user(GameId(game.id), channel_id, name);

            let content = if let Err(e) = ret {
                tracing::error!("{e}");

                format!("Failed to unmute '{name}'.")
            } else {
                format!("The user '{name}' is now unmuted for '{}'.", game.name)
            };

            content.into()
        }
        None => "Game not found.".into(),
    };

    update_response_content(ctx, interaction, &content).await
}

async fn find_game_mod(
    modio: Modio,
    game_filter: Filter,
    mod_filter: Filter,
) -> Result<(Option<Game>, Option<Mod>), Error> {
    let Some(game) = modio.games().search(game_filter).first().await? else {
        return Ok((None, None));
    };

    let mod_ = modio
        .game(game.id)
        .mods()
        .search(mod_filter)
        .first()
        .await?;

    Ok((Some(game), mod_))
}

const fn subs_suffix(evts: Events) -> &'static str {
    match (evts.contains(Events::NEW), evts.contains(Events::UPD)) {
        (true, true) | (false, false) => " (+Δ)",
        (true, false) => " (+)",
        (false, true) => " (Δ)",
    }
}

fn push_tags<'a, I>(s: &mut String, iter: I)
where
    I: std::iter::Iterator<Item = &'a String>,
{
    let mut iter = iter.peekable();
    while let Some(t) = iter.next() {
        s.push('`');
        s.push_str(t);
        s.push('`');
        if iter.peek().is_some() {
            s.push_str(", ");
        }
    }
}
