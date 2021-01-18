use std::collections::{BTreeSet, HashSet};
use std::future::Future;
use std::time::Duration;

use futures_util::TryStreamExt;
use modio::filter::prelude::*;
use modio::games::{ApiAccessOptions, Game};
use modio::mods::filters::events::EventType as EventTypeFilter;
use modio::mods::{EventType, Mod};
use modio::Modio;
use serenity::builder::CreateMessage;
use serenity::prelude::*;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tracing::{debug, error, trace};

use crate::commands::prelude::*;
use crate::db::Subscriptions;
use crate::metrics::Metrics;
use crate::util;

const MIN: Duration = Duration::from_secs(60);
const INTERVAL_DURATION: Duration = Duration::from_secs(300);

pub fn task(client: &Client, modio: Modio, metrics: Metrics) -> impl Future<Output = ()> {
    let data = client.data.clone();
    let http = client.cache_and_http.http.clone();
    let (tx, mut rx) = mpsc::channel::<(BTreeSet<ChannelId>, CreateMessage<'_>)>(100);

    tokio::spawn(async move {
        loop {
            if let Some((channels, msg)) = rx.recv().await {
                metrics.notifications.inc_by(channels.len() as u64);
                for channel in channels {
                    let mut msg = msg.clone();
                    if let Err(e) = channel.send_message(&http, |_| &mut msg).await {
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
        let mut interval = tokio::time::interval_at(Instant::now() + MIN, INTERVAL_DURATION);

        loop {
            let tstamp = tstamp.take().unwrap_or_else(util::current_timestamp);
            interval.tick().await;

            let filter = DateAdded::gt(tstamp)
                .and(EventTypeFilter::_in(vec![
                    EventType::ModfileChanged,
                    // EventType::ModEdited,
                    EventType::ModDeleted,
                    EventType::ModAvailable,
                    EventType::ModUnavailable,
                ]))
                .order_by(Id::asc());

            let data = data.read().await;
            let subs = data
                .get::<Subscriptions>()
                .expect("failed to get subscriptions")
                .load()
                .unwrap_or_else(|e| {
                    error!("failed to load subscriptions: {}", e);
                    Default::default()
                });

            for (game, channels) in subs {
                if channels.is_empty() {
                    continue;
                }
                debug!(
                    "polling events at {} for game={} channels: {:?}",
                    tstamp, game, channels
                );
                let tx = tx.clone();
                let filter = filter.clone();
                let game = modio.game(game);
                let mods = game.mods();

                let task = async move {
                    use std::collections::BTreeMap;
                    type Events = BTreeMap<u32, Vec<(u32, EventType)>>;

                    // - Group the events by mod
                    // - Filter `MODFILE_CHANGED` events for new mods
                    // - Ungroup the events ordered by event id

                    let mut events = mods
                        .events(filter)
                        .iter()
                        .await?
                        .try_fold(Events::new(), |mut events, e| async {
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
                    for (_, evt) in events.iter_mut() {
                        use EventType::*;
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
                        for (eid, t) in *evt {
                            updates.insert(eid, (m, t));
                        }
                    }

                    let game = game.get().await?;

                    for (_, (m, evt)) in updates.into_iter() {
                        let mut msg = CreateMessage::default();
                        create_message(&game, m, evt, &mut msg);
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
                                    debug!("mod ignored #{}: {} for {:?}", channel, evt, m.name);
                                    trace!("mod tags: {:?}; sub tags: {:?}", mod_tags, tags);
                                    continue;
                                }
                            }
                            effected_channels.insert(*channel);
                        }
                        debug!(
                            "send message {} for {:?} to {:?}",
                            evt, m.name, effected_channels
                        );
                        if let Err(e) = tx.send((effected_channels, msg)).await {
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

fn create_message<'a, 'b>(
    game: &Game,
    mod_: &Mod,
    event: &EventType,
    m: &'b mut CreateMessage<'a>,
) -> &'b mut CreateMessage<'a> {
    use crate::commands::mods::ModExt;

    let create_embed =
        |m: &'b mut CreateMessage<'a>, desc: &str, changelog: Option<(&str, String, bool)>| {
            m.embed(|e| {
                e.title(&mod_.name)
                    .url(&mod_.profile_url)
                    .description(desc)
                    .thumbnail(&mod_.logo.thumb_320x180)
                    .author(|a| {
                        a.name(&game.name)
                            .icon_url(&game.icon.thumb_64x64.to_string())
                            .url(&game.profile_url.to_string())
                    })
                    .footer(|f| mod_.submitted_by.create_footer(f))
                    .fields(changelog)
            })
        };

    let with_ddl = game
        .api_access_options
        .contains(ApiAccessOptions::ALLOW_DIRECT_DOWNLOAD);

    match event {
        EventType::ModEdited => create_embed(m, "The mod has been edited.", None),
        EventType::ModAvailable => {
            let m = m.content("A new mod is available. :tada:");
            mod_.create_new_mod_message(game, m)
        }
        EventType::ModUnavailable => create_embed(m, "The mod is now unavailable.", None),
        EventType::ModfileChanged => {
            let (desc, changelog) = mod_
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
                            let pos = it.last().unwrap_or_else(|| c.len());
                            ("Changelog", c[..pos].to_owned(), true)
                        });
                    let desc = format!("A new version is available. {}", download);

                    (desc, changelog)
                })
                .unwrap_or_default();
            create_embed(m, &desc, changelog)
        }
        EventType::ModDeleted => create_embed(m, "The mod has been permanently deleted.", None),
        _ => create_embed(m, "event ignored", None),
    }
}
