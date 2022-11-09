use std::borrow::Cow;
use std::fmt::Write;

use futures_util::TryStreamExt;
use modio::filter::prelude::*;
use modio::games::{ApiAccessOptions, Game};
use modio::mods::filters::Popular;
use modio::mods::{Mod, Statistics};
use serde::{Deserialize, Serialize};
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::component::button::ButtonStyle;
use twilight_model::application::component::{ActionRow, Button, Component};
use twilight_model::application::interaction::application_command::{
    CommandData, CommandDataOption, CommandOptionValue,
};
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::embed::{Embed, EmbedField};
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::command::{CommandBuilder, StringBuilder};
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, ImageSource,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use super::{create_response, EphemeralMessage, InteractionResponseBuilderExt};
use crate::bot::Context;
use crate::error::Error;
use crate::util::format_timestamp;

pub fn commands() -> Vec<Command> {
    vec![
        CommandBuilder::new(
            "mods",
            "List mods or show the details for a single mod.",
            CommandType::ChatInput,
        )
        .dm_permission(false)
        .option(StringBuilder::new("mod", "ID or search"))
        .option(StringBuilder::new("game", "ID or search"))
        .build(),
        CommandBuilder::new("popular", "List popular mods.", CommandType::ChatInput)
            .dm_permission(false)
            .option(StringBuilder::new(
                "game",
                "ID or search game instead of the default game.",
            ))
            .build(),
    ]
}

pub async fn list(
    ctx: &Context,
    interaction: &Interaction,
    command: &CommandData,
) -> Result<(), Error> {
    let mut search = None;
    let mut game_id = None;
    for opt in &command.options {
        match &opt.value {
            CommandOptionValue::String(s) if opt.name == "mod" => {
                search = Some(s);
            }
            CommandOptionValue::String(s) if opt.name == "game" => {
                let game = search_game(ctx, s).await?;

                if game.is_none() {
                    let data = "Game not found.".into_ephemeral();
                    return create_response(ctx, interaction, data).await;
                }
                game_id = game.map(|g| g.id);
            }
            _ => {}
        }
    }

    let game_id = game_id.or_else(|| {
        let settings = ctx.settings.lock().unwrap();
        interaction.guild_id.and_then(|id| settings.game(id.get()))
    });

    let mut builder = InteractionResponseDataBuilder::new();
    if let Some(id) = game_id {
        let (filter, title): (Filter, Cow<'_, _>) = if let Some(search) = search {
            match search.parse::<u32>() {
                Ok(id) => (Id::eq(id), "Mods".into()),
                Err(_) => (
                    Fulltext::eq(search),
                    format!("Mods matching: '{}'", search).into(),
                ),
            }
        } else {
            (Filter::default(), "Mods".into())
        };
        let game = ctx.modio.game(id);
        let mods = game.mods();

        let first_page = mods
            .search(filter.limit(20))
            .paged()
            .await?
            .try_next()
            .await?;

        if let Some(page) = first_page {
            match page.as_slice() {
                [mod_] => {
                    let game = game.get().await?;
                    let embed = create_mod_embed(&game, mod_).build();
                    builder = builder.embeds([embed]);
                }
                list => {
                    let embed = create_list_embed(list, &title, page.current(), page.page_count());

                    let components = if page.total() > page.len() {
                        Some(create_browse_buttons(
                            id,
                            search.map(String::as_str),
                            0,
                            20,
                            page.current(),
                            page.page_count(),
                        ))
                    } else {
                        None
                    };
                    builder = builder.embeds([embed]).components(components);
                }
            }
        } else {
            builder = builder.ephemeral("no mods found.");
        }
    } else {
        builder = builder.ephemeral("default game is not set.");
    }

    create_response(ctx, interaction, builder.build()).await
}

#[derive(Deserialize, Serialize)]
struct CustomId<'a> {
    #[serde(rename = "b")]
    button: &'a str,
    #[serde(rename = "g")]
    game_id: u32,
    #[serde(rename = "q")]
    search: Option<&'a str>,
    #[serde(rename = "o")]
    offset: usize,
    #[serde(rename = "l")]
    limit: usize,
    #[serde(rename = "s")]
    sort: Option<&'a str>,
}

pub async fn list_component(
    ctx: &Context,
    interaction: &Interaction,
    component: &MessageComponentInteractionData,
) -> Result<(), Error> {
    let CustomId {
        game_id,
        search,
        offset,
        limit,
        ..
    } = serde_urlencoded::from_str(&component.custom_id).unwrap();

    let (filter, title): (Filter, Cow<'_, _>) = if let Some(search) = search {
        (
            Fulltext::eq(search),
            format!("Mods matching: '{}'", search).into(),
        )
    } else {
        (Filter::default(), "Mods".into())
    };
    let filter = filter.offset(offset).limit(20);
    let game = ctx.modio.game(game_id);
    let mods = game.mods();

    let page = mods.search(filter).paged().await?.try_next().await?;

    let mut builder = InteractionResponseDataBuilder::new();
    if let Some(page) = page {
        let embed = create_list_embed(&page, &title, page.current(), page.page_count());
        let components = create_browse_buttons(
            game_id,
            search,
            offset,
            limit,
            page.current(),
            page.page_count(),
        );
        builder = builder.embeds([embed]).components([components]);
    }

    ctx.interaction()
        .create_response(
            interaction.id,
            &interaction.token,
            &InteractionResponse {
                kind: InteractionResponseType::UpdateMessage,
                data: Some(builder.build()),
            },
        )
        .exec()
        .await?;

    Ok(())
}

pub async fn popular(
    ctx: &Context,
    interaction: &Interaction,
    command: &CommandData,
) -> Result<(), Error> {
    let game_id = match command.options.as_slice() {
        [CommandDataOption {
            value: CommandOptionValue::String(s),
            ..
        }] => {
            let game = search_game(ctx, s).await?;
            if game.is_none() {
                let data = "Game not found.".into_ephemeral();
                return create_response(ctx, interaction, data).await;
            }
            game.map(|g| g.id)
        }
        _ => None,
    };

    let game_id = game_id.or_else(|| {
        let settings = ctx.settings.lock().unwrap();
        interaction.guild_id.and_then(|id| settings.game(id.get()))
    });

    let mut builder = InteractionResponseDataBuilder::new();
    if let Some(id) = game_id {
        let filter = with_limit(10).order_by(Popular::desc());
        let game = ctx.modio.game(id);
        let mods = game.mods().search(filter).first_page().await?;
        let game = game.get().await?;

        if mods.is_empty() {
            builder = builder.ephemeral("no mods founds.");
        } else {
            let mut content = String::new();
            for mod_ in mods {
                let _ = writeln!(
                    content,
                    "{:02}. [{}]({}) ({}) +{}/-{}",
                    mod_.stats.popularity.rank_position,
                    mod_.name,
                    mod_.profile_url,
                    mod_.id,
                    mod_.stats.ratings.positive,
                    mod_.stats.ratings.negative,
                );
            }

            let embed = EmbedBuilder::new()
                .title("Popular Mods")
                .description(content)
                .author(
                    EmbedAuthorBuilder::new(game.name)
                        .url(game.profile_url)
                        .icon_url(ImageSource::url(game.icon.thumb_64x64.to_string()).unwrap()),
                )
                .build();

            builder = builder.embeds([embed]);
        }
    } else {
        builder = builder.ephemeral("default game is not set.");
    }

    create_response(ctx, interaction, builder.build()).await
}

async fn search_game(ctx: &Context, search: &str) -> Result<Option<Game>, Error> {
    let filter = match search.parse::<u32>() {
        Ok(id) => Id::eq(id),
        Err(_) => Fulltext::eq(search),
    };
    let game = ctx.modio.games().search(filter).first().await?;
    Ok(game)
}

fn create_list_embed(mods: &[Mod], title: &str, page: usize, page_count: usize) -> Embed {
    let mut content = String::new();
    for mod_ in mods {
        let _ = writeln!(content, "{}. {}", mod_.id, mod_.name);
    }
    EmbedBuilder::new()
        .title(title)
        .description(content)
        .footer(EmbedFooterBuilder::new(format!(
            "Page: {}/{}",
            page, page_count,
        )))
        .build()
}

fn create_browse_buttons(
    game_id: u32,
    search: Option<&'_ str>,
    offset: usize,
    limit: usize,
    page: usize,
    page_count: usize,
) -> Component {
    let custom_id = CustomId {
        button: "prev",
        game_id,
        search,
        offset: offset.saturating_sub(limit),
        limit,
        sort: None,
    };
    let prev = Button {
        custom_id: Some(serde_urlencoded::to_string(&custom_id).unwrap()),
        style: ButtonStyle::Primary,
        label: Some("prev".to_owned()),
        disabled: page == 1,
        emoji: None,
        url: None,
    };
    let custom_id = CustomId {
        button: "next",
        offset: offset + limit,
        ..custom_id
    };
    let next = Button {
        custom_id: Some(serde_urlencoded::to_string(&custom_id).unwrap()),
        style: ButtonStyle::Primary,
        label: Some("next".to_owned()),
        disabled: page == page_count,
        emoji: None,
        url: None,
    };
    let row = ActionRow {
        components: vec![prev.into(), next.into()],
    };
    row.into()
}

fn create_mod_embed(game: &Game, mod_: &Mod) -> EmbedBuilder {
    let with_ddl = game
        .api_access_options
        .contains(ApiAccessOptions::ALLOW_DIRECT_DOWNLOAD);

    let mut footer = EmbedFooterBuilder::new(&mod_.submitted_by.username);
    if let Some(avatar) = &mod_.submitted_by.avatar {
        footer = footer.icon_url(ImageSource::url(avatar.thumb_50x50.to_string()).unwrap());
    }

    let builder = EmbedBuilder::new()
        .title(&mod_.name)
        .url(mod_.profile_url.to_string())
        .description(&mod_.summary)
        .thumbnail(ImageSource::url(mod_.logo.thumb_320x180.to_string()).unwrap())
        .author(
            EmbedAuthorBuilder::new(&game.name)
                .url(game.profile_url.to_string())
                .icon_url(ImageSource::url(game.icon.thumb_64x64.to_string()).unwrap()),
        )
        .footer(footer);

    create_fields(builder, mod_, false, with_ddl)
}

pub fn create_fields(
    mut builder: EmbedBuilder,
    m: &Mod,
    is_new: bool,
    with_ddl: bool,
) -> EmbedBuilder {
    fn ratings(stats: &Statistics) -> EmbedField {
        EmbedField {
            name: "Rating".to_owned(),
            value: format!(
                r#"Rank: {}/{}
Downloads: {}
Subscribers: {}
Votes: +{}/-{}"#,
                stats.popularity.rank_position,
                stats.popularity.rank_total,
                stats.downloads_total,
                stats.subscribers_total,
                stats.ratings.positive,
                stats.ratings.negative,
            ),
            inline: true,
        }
    }
    #[allow(clippy::cast_possible_wrap)]
    fn dates(m: &Mod) -> EmbedField {
        let added = format_timestamp(m.date_added as i64);
        let updated = format_timestamp(m.date_updated as i64);
        EmbedField {
            name: "Dates".to_owned(),
            value: format!("Created: {}\nUpdated: {}", added, updated),
            inline: true,
        }
    }
    fn info(m: &Mod, with_ddl: bool) -> Option<EmbedField> {
        let mut info = if with_ddl {
            String::from("Links: ")
        } else {
            String::new()
        };
        if let Some(homepage) = &m.homepage_url {
            let _ = write!(info, "[Homepage]({}), ", homepage);
        }
        if let Some(f) = &m.modfile {
            if with_ddl {
                let _ = writeln!(info, "[Download]({})", f.download.binary_url);
            }
            if let Some(version) = &f.version {
                let _ = writeln!(info, "Version: {}", version);
            }
            let _ = writeln!(info, "Size: {}", bytesize::to_string(f.filesize, false));
        }
        if info.len() > 7 {
            Some(EmbedField {
                name: "Info".to_owned(),
                value: info,
                inline: true,
            })
        } else {
            None
        }
    }
    fn tags(m: &Mod) -> Option<EmbedField> {
        if m.tags.is_empty() {
            return None;
        }
        let tags = m
            .tags
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        Some(EmbedField {
            name: "Tags".to_owned(),
            value: tags,
            inline: true,
        })
    }

    if is_new {
        if let Some(field) = info(m, with_ddl) {
            builder = builder.field(field);
        }
        if let Some(field) = tags(m) {
            builder = builder.field(field);
        }
    } else {
        builder = builder.field(ratings(&m.stats));
        if let Some(field) = info(m, with_ddl) {
            builder = builder.field(field);
        }
        builder = builder.field(dates(m));
        if let Some(field) = tags(m) {
            builder = builder.field(field);
        }
    }
    builder
}
