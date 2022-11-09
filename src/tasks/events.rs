use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::future::Future;
use std::time::Duration;

use futures_util::stream::FuturesUnordered;
use futures_util::{StreamExt, TryStreamExt};
use modio::filter::prelude::*;
use modio::games::{ApiAccessOptions, Game};
use modio::mods::filters::events::EventType as EventTypeFilter;
use modio::mods::{EventType, Mod};
use tokio::sync::mpsc;
use tokio::time::{self, Instant};
use tracing::{debug, error, trace};
use twilight_model::channel::embed::Embed;
use twilight_model::id::Id as ChannelId;
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder, ImageSource,
};

use crate::bot::Context;
use crate::commands::mods::create_fields;
use crate::util;

const MIN: Duration = Duration::from_secs(60);
const INTERVAL_DURATION: Duration = Duration::from_secs(300);

#[allow(clippy::too_many_lines)]
pub fn task(ctx: Context) -> impl Future<Output = ()> {
    let (sender, mut receiver) = mpsc::channel::<(BTreeSet<u64>, Option<String>, Embed)>(100);

    tokio::spawn(async move {
        loop {
            if let Some((channels, content, embed)) = receiver.recv().await {
                ctx.metrics.notifications.inc_by(channels.len() as u64);

                let embeds = [embed];
                let mut cc = channels
                    .into_iter()
                    .map(|id| {
                        let mut msg = ctx
                            .client
                            .create_message(ChannelId::new(id))
                            .embeds(&embeds)
                            .unwrap();
                        if let Some(content) = &content {
                            msg = msg.content(content).unwrap();
                        }
                        msg.exec()
                    })
                    .collect::<FuturesUnordered<_>>();

                while let Some(ret) = cc.next().await {
                    if let Err(e) = ret {
                        error!("{}", e);
                    }
                }
            }
        }
    });

    let mut tstamp = std::env::var("MODIO_DEBUG_TIMESTAMP")
        .ok()
        .and_then(|v| v.parse::<u64>().ok());

    async move {
        let mut interval = time::interval_at(Instant::now() + MIN, INTERVAL_DURATION);

        loop {
            let tstamp = tstamp.take().unwrap_or_else(util::current_timestamp);
            interval.tick().await;

            let filter = DateAdded::gt(tstamp)
                .and(EventTypeFilter::_in(vec![
                    EventType::ModfileChanged,
                    EventType::ModDeleted,
                    EventType::ModAvailable,
                    EventType::ModUnavailable,
                ]))
                .order_by(Id::asc());

            let subs = ctx.subscriptions.load().unwrap_or_else(|e| {
                error!("failed to load subscriptions: {}", e);
                HashMap::default()
            });

            for (game, channels) in subs {
                if channels.is_empty() {
                    continue;
                }
                debug!(
                    "polling events at {} for game={} channels: {:?}",
                    tstamp, game, channels
                );
                let sender = sender.clone();
                let filter = filter.clone();
                let game = ctx.modio.game(game);
                let mods = game.mods();

                let task = async move {
                    type Events = BTreeMap<u32, Vec<(u32, EventType)>>;

                    // - Group the events by mod
                    // - Filter `MODFILE_CHANGED` events for new mods
                    // - Ungroup the events ordered by event id

                    let mut events = mods
                        .events(filter)
                        .iter()
                        .await?
                        .try_fold(Events::new(), |mut events, e| async move {
                            events
                                .entry(e.mod_id)
                                .or_default()
                                .push((e.id, e.event_type));
                            Ok(events)
                        })
                        .await?;

                    if events.is_empty() {
                        return Ok(());
                    }

                    // Filter `MODFILE_CHANGED` events for new mods
                    for evt in &mut events.values_mut() {
                        use EventType::{ModAvailable, ModfileChanged};
                        if evt.iter().any(|(_, t)| t == &ModAvailable) {
                            let pos = evt.iter().position(|(_, t)| t == &ModfileChanged);
                            if let Some(pos) = pos {
                                evt.remove(pos);
                            }
                        }
                    }

                    // Load the mods for the events
                    let filter = Id::_in(events.keys().collect::<Vec<_>>());
                    let events = game
                        .mods()
                        .search(filter)
                        .iter()
                        .await?
                        .map_ok(|m| events.get(&m.id).map(|evt| (m, evt)))
                        .try_filter_map(|e| async { Ok(e) })
                        .try_collect::<Vec<_>>()
                        .await?;

                    // Ungroup the events ordered by event id
                    let mut updates = BTreeMap::new();
                    for (m, evt) in &events {
                        for (event_id, event_type) in *evt {
                            updates.insert(event_id, (m, event_type));
                        }
                    }

                    let game = game.get().await?;

                    for (_, (m, evt)) in updates {
                        let mut effected_channels = BTreeSet::new();

                        for (channel, tags, _, evts, excluded_mods, excluded_users) in &channels {
                            if *evt == EventType::ModAvailable
                                && !evts.contains(crate::db::Events::NEW)
                                || *evt == EventType::ModfileChanged
                                    && !evts.contains(crate::db::Events::UPD)
                            {
                                debug!("event ignored #{}: {} for {:?}", channel, evt, m.name,);
                                continue;
                            }
                            if excluded_users.contains(&m.submitted_by.username)
                                || excluded_users.contains(&m.submitted_by.name_id)
                            {
                                debug!(
                                    "user ignored #{}: {} for {:?}/{:?}",
                                    channel, evt, m.submitted_by.name_id, m.name,
                                );
                                continue;
                            }
                            if excluded_mods.contains(&m.id) {
                                debug!("mod ignored #{}: {} for {:?}", channel, evt, m.name,);
                                continue;
                            }
                            if !tags.is_empty() {
                                let mod_tags = m.tags.iter().map(|t| t.name.as_str()).collect();

                                // Hidden tags are saved with a leading `*`
                                let tags: HashSet<_> =
                                    tags.iter().map(|t| t.trim_start_matches('*')).collect();
                                if !tags.is_subset(&mod_tags) {
                                    debug!(
                                        "mod ignored based on tags #{}: {} for {:?}",
                                        channel, evt, m.name
                                    );
                                    trace!("mod tags: {:?}; sub tags: {:?}", mod_tags, tags);
                                    continue;
                                }
                            }
                            effected_channels.insert(*channel);
                        }
                        if effected_channels.is_empty() {
                            debug!("no channels left to send to");
                            continue;
                        }

                        debug!(
                            "send message {} for {:?} to {:?}",
                            evt, m.name, effected_channels
                        );
                        let (content, embed) = create_mod_message(&game, m, evt);
                        if let Err(e) = sender.send((effected_channels, content, embed)).await {
                            error!("{}", e);
                        }
                    }
                    Ok::<_, modio::Error>(())
                };

                tokio::spawn(async {
                    if let Err(e) = task.await {
                        error!("{}", e);
                    }
                });
            }
        }
    }
}

fn create_mod_message(game: &Game, mod_: &Mod, event_type: &EventType) -> (Option<String>, Embed) {
    let with_ddl = game
        .api_access_options
        .contains(ApiAccessOptions::ALLOW_DIRECT_DOWNLOAD);

    let embed = match event_type {
        EventType::ModEdited => create_embed(game, mod_, "The mod has been edited.", false),
        EventType::ModAvailable => {
            let content = "A new mod is available. :tada:".to_owned();
            let embed = create_embed(game, mod_, &mod_.summary, true);
            let embed = create_fields(embed, mod_, true, with_ddl).build();
            return (Some(content), embed);
        }
        EventType::ModUnavailable => create_embed(game, mod_, "The mod is now unavailable.", false),
        EventType::ModfileChanged => {
            let (download, changelog) = mod_
                .modfile
                .as_ref()
                .map(|f| {
                    let link = &f.download.binary_url;
                    let no_version = || {
                        if with_ddl {
                            format!("[Download]({})", link)
                        } else {
                            String::new()
                        }
                    };
                    let version = |v| {
                        if with_ddl {
                            format!("[Version {}]({})", v, link)
                        } else {
                            format!("Version {}", v)
                        }
                    };
                    let download = f
                        .version
                        .as_ref()
                        .filter(|v| !v.is_empty())
                        .map_or_else(no_version, version);
                    let changelog = f
                        .changelog
                        .as_ref()
                        .map(util::strip_html_tags)
                        .filter(|c| !c.is_empty())
                        .map(|c| {
                            let it = c.char_indices().rev().scan(c.len(), |state, (pos, _)| {
                                if *state > 1024 {
                                    *state = pos;
                                    Some(pos)
                                } else {
                                    None
                                }
                            });
                            let pos = it.last().unwrap_or(c.len());
                            EmbedFieldBuilder::new("Changelog", c[..pos].to_owned()).inline()
                        });
                    (download, changelog)
                })
                .unwrap_or_default();

            let desc = format!("A new version is available. {}", download);
            let mut embed = create_embed(game, mod_, &desc, false);
            if let Some(changelog) = changelog {
                embed = embed.field(changelog);
            }
            embed
        }
        EventType::ModDeleted => {
            create_embed(game, mod_, "The mod has been permanently deleted.", false)
        }
        _ => create_embed(game, mod_, "event ignored", false),
    };

    (None, embed.build())
}

fn create_embed(game: &Game, mod_: &Mod, desc: &str, big_thumbnail: bool) -> EmbedBuilder {
    let mut footer = EmbedFooterBuilder::new(mod_.submitted_by.username.clone());
    if let Some(avatar) = &mod_.submitted_by.avatar {
        footer = footer.icon_url(ImageSource::url(avatar.thumb_50x50.to_string()).unwrap());
    }

    let embed = EmbedBuilder::new()
        .title(mod_.name.clone())
        .url(mod_.profile_url.to_string())
        .description(desc)
        .author(
            EmbedAuthorBuilder::new(game.name.clone())
                .url(game.profile_url.to_string())
                .icon_url(ImageSource::url(game.icon.thumb_64x64.to_string()).unwrap()),
        )
        .footer(footer);

    if big_thumbnail {
        embed.image(ImageSource::url(mod_.logo.thumb_640x360.to_string()).unwrap())
    } else {
        embed.thumbnail(ImageSource::url(mod_.logo.thumb_320x180.to_string()).unwrap())
    }
}
