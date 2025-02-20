use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::fmt::{Display, Write};

use futures_util::stream::FuturesUnordered;
use modio::filter::prelude::*;
use modio::types::games::{ApiAccessOptions, Game};
use modio::types::mods::Mod;
use modio::Modio;
use tokio_stream::StreamExt;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::application_command::{
    CommandData, CommandDataOption, CommandOptionValue,
};
use twilight_model::application::interaction::{Interaction, InteractionContextType};
use twilight_model::guild::Permissions;
use twilight_util::builder::command::{
    CommandBuilder, IntegerBuilder, StringBuilder, SubCommandBuilder, SubCommandGroupBuilder,
};
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder};

use super::{
    autocomplete_games, create_response, defer_ephemeral, search_game, update_response_content,
    update_response_from_content, AutocompleteExt, EphemeralMessage, InteractionExt, SubCommandExt,
};
use crate::bot::Context;
use crate::db::types::{ChannelId, GameId, ModId};
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
    .contexts([InteractionContextType::Guild])
    .default_member_permissions(Permissions::MANAGE_CHANNELS)
    .build()]
}

pub async fn handle_command(
    ctx: &Context,
    interaction: &Interaction,
    command: &CommandData,
) -> Result<(), Error> {
    if let Some(("game", value)) = command.autocomplete() {
        return autocomplete_games(ctx, interaction, value).await;
    }

    match command.subcommand() {
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
    let guild_id = interaction.guild_id().unwrap();

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
        .map(|g| (g.id.get(), g.name))
        .collect::<HashMap<_, _>>();

    let mut embed = EmbedBuilder::new().title("Subscriptions");

    let mut content = String::new();
    for (channel_id, subs) in subs {
        _ = writeln!(&mut content, "__Channel:__ <#{channel_id}>");
        for (game_id, tags, evts) in subs {
            if let Some(game) = games.get(&game_id.get()) {
                _ = write!(&mut content, "`{game_id}.` {game}");
            } else {
                _ = write!(&mut content, "{game_id}");
            }
            content.push_str(evts.to_suffix());

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
        embed = embed.field(EmbedFieldBuilder::new(
            "Muted mods",
            to_content(&games, excluded_mods),
        ));
    }

    if !excluded_users.is_empty() {
        embed = embed.field(EmbedFieldBuilder::new(
            "Muted users",
            to_content(&games, excluded_users),
        ));
    }
    ctx.interaction()
        .update_response(&interaction.token)
        .embeds(Some(&[embed.build()]))
        .await?;

    Ok(())
}

fn to_content<I, E, D>(games: &HashMap<u64, String>, excluded: I) -> String
where
    I: IntoIterator<Item = ((GameId, ChannelId), E)>,
    E: IntoIterator<Item = D>,
    D: Display,
{
    let excluded = excluded
        .into_iter()
        .map(|((game_id, channel_id), items)| (channel_id, (game_id, items)))
        .fold(BTreeMap::<_, Vec<_>>::new(), |mut map, (key, value)| {
            map.entry(key).or_default().push(value);
            map
        });

    let mut content = String::new();
    for (channel_id, entries) in excluded {
        _ = writeln!(&mut content, "__Channel:__ <#{channel_id}>");
        for (game_id, items) in entries {
            if let Some(game) = games.get(&game_id.get()) {
                _ = write!(&mut content, "`{game_id}.` {game}: ");
            } else {
                _ = write!(&mut content, "{game_id}: ");
            }
            let mut it = items.into_iter().peekable();
            while let Some(item) = it.next() {
                _ = write!(&mut content, "`{item}`");
                if it.peek().is_some() {
                    content.push_str(", ");
                }
            }
            content.push('\n');
        }
    }
    content
}

async fn list(ctx: &Context, interaction: &Interaction) -> Result<(), Error> {
    let channel_id = interaction.channel_id().unwrap();
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
        _ = write!(&mut content, "`{game_id}.` {name}");

        content.push_str(evts.to_suffix());

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
        .embeds(Some(&[embed]))
        .await?;

    Ok(())
}

async fn subscribe(
    ctx: &Context,
    interaction: &Interaction,
    opts: &[CommandDataOption],
) -> Result<(), Error> {
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

    let channel_id = interaction.channel_id().unwrap();
    let guild_id = interaction.guild_id().unwrap();

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

    let game_id = GameId(game.id);
    let ret = ctx
        .subscriptions
        .add(game_id, channel_id, sub_tags, guild_id, evts);

    let content: Cow<'_, str> = match ret {
        Ok(()) => format!("Subscribed to '{}'.", game.name).into(),
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
    let channel_id = interaction.channel_id().unwrap();

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

    let game_id = GameId(game.id);
    let ret = ctx
        .subscriptions
        .remove(game_id, channel_id, sub_tags, evts);

    let content: Cow<'_, str> = match ret {
        Ok(()) => format!("Unsubscribed from '{}'.", game.name).into(),
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
    defer_ephemeral(ctx, interaction).await?;

    match opts.subcommand() {
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
    _: &[CommandDataOption],
) -> Result<(), Error> {
    let channel_id = interaction.channel_id().unwrap();
    let excluded = ctx.subscriptions.list_excluded_mods(channel_id)?;

    let muted = match excluded.len() {
        0 => {
            let content = "No mod is muted.";
            return update_response_content(ctx, interaction, content).await;
        }
        1 => {
            let (GameId(game), mods) = excluded.into_iter().next().unwrap();
            let filter = Id::_in(mods.into_iter().collect::<Vec<_>>());
            let mut st = ctx.modio.game(game).mods().search(filter).iter().await?;

            let mut buf = ContentBuilder::new(4000);
            while let Some(mod_) = st.try_next().await? {
                _ = writeln!(&mut buf, "`{}.` {}", mod_.id, mod_.name);
            }
            buf
        }
        _ => {
            let mut st = excluded
                .into_iter()
                .map(|(GameId(game), mods)| async move {
                    let filter = Id::_in(mods.into_iter().collect::<Vec<_>>());
                    tokio::try_join!(
                        ctx.modio.game(game).get(),
                        ctx.modio.game(game).mods().search(filter).collect(),
                    )
                })
                .collect::<FuturesUnordered<_>>();

            let mut buf = ContentBuilder::new(4000);
            while let Some((game, mods)) = st.try_next().await? {
                _ = writeln!(&mut buf, "**{}**", game.name);
                for m in mods {
                    _ = writeln!(&mut buf, "`{}.` {}", m.id, m.name);
                }
                _ = writeln!(&mut buf);
            }
            buf
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

    let game_mod = find_game_mod(&ctx.modio, game_filter, mod_filter).await?;

    let content: Cow<'_, str> = match game_mod {
        (None, _) => "Game not found.".into(),
        (_, None) => "Mod not found.".into(),
        (Some(game), Some(mod_)) => {
            let channel_id = interaction.channel_id().unwrap();
            let guild_id = interaction.guild_id().unwrap();

            let game_id = GameId(game.id);
            let mod_id = ModId(mod_.id);
            let ret = ctx
                .subscriptions
                .mute_mod(game_id, channel_id, guild_id, mod_id);

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

    let game_mod = find_game_mod(&ctx.modio, game_filter, mod_filter).await?;

    let content: Cow<'_, str> = match game_mod {
        (None, _) => "Game not found.".into(),
        (_, None) => "Mod not found.".into(),
        (Some(game), Some(mod_)) => {
            let channel_id = interaction.channel_id().unwrap();

            let game_id = GameId(game.id);
            let mod_id = ModId(mod_.id);
            let ret = ctx.subscriptions.unmute_mod(game_id, channel_id, mod_id);

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
    defer_ephemeral(ctx, interaction).await?;

    match opts.subcommand() {
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
    _: &[CommandDataOption],
) -> Result<(), Error> {
    let channel_id = interaction.channel_id().unwrap();
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
                _ = writeln!(&mut muted, "`{}.` {name}", i + 1);
            }
            muted
        }
        _ => {
            let mut st = excluded
                .into_iter()
                .map(|(GameId(game), users)| async move {
                    let game = ctx.modio.game(game).get().await?;
                    Ok::<_, Error>((game, users))
                })
                .collect::<FuturesUnordered<_>>();

            let mut buf = ContentBuilder::new(4000);
            while let Some((game, users)) = st.try_next().await? {
                _ = writeln!(&mut buf, "**{}**", game.name);
                for (i, name) in users.iter().enumerate() {
                    _ = writeln!(&mut buf, "`{}.` {name}", i + 1);
                }
                _ = writeln!(&mut buf);
            }
            buf
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
            let guild_id = interaction.guild_id().unwrap();
            let channel_id = interaction.channel_id().unwrap();

            let game_id = GameId(game.id);
            let ret = ctx
                .subscriptions
                .mute_user(game_id, channel_id, guild_id, name);

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
            let channel_id = interaction.channel_id().unwrap();

            let game_id = GameId(game.id);
            let ret = ctx.subscriptions.unmute_user(game_id, channel_id, name);

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
    modio: &Modio,
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

impl Events {
    const fn to_suffix(self) -> &'static str {
        match (self.contains(Events::NEW), self.contains(Events::UPD)) {
            (true, true) | (false, false) => " (+Δ)",
            (true, false) => " (+)",
            (false, true) => " (Δ)",
        }
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
