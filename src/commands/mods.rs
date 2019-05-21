use futures::future;
use modio::filter::prelude::*;
use modio::games::Game;
use modio::mods::filters::Popular as PopularFilter;
use modio::mods::{Mod, Statistics};

use crate::commands::prelude::*;
use crate::util::ContentBuilder;

command!(
    ListMods(self, ctx, msg) {
        let channel = msg.channel_id;
        let game_id = msg.guild_id.and_then(|id| {
            Settings::game(ctx, id)
        });
        if let Some(id) = game_id {
            let opts = Default::default();
            let task = list_mods(
                self.modio.game(id).mods(),
                &opts,
                None,
                channel,
            );

            self.executor.spawn(task);
        } else {
            let _ = channel.say("default game is not set.");
        }
    }

    options(opts) {
        opts.desc = Some("List mods of the default game".to_string());
        opts.usage = Some("mods".to_string());
        opts.guild_only = true;
        opts.bucket = Some("simple".to_string());
        opts.max_args = Some(0);
    }
);

command!(
    ModInfo(self, ctx, msg, args) {
        let channel = msg.channel_id;
        let game_id = msg.guild_id.and_then(|id| {
            Settings::game(ctx, id)
        });
        if let Some(game_id) = game_id {
            let filter = match args.single::<u32>() {
                Ok(id) => Id::eq(id),
                Err(_) => Fulltext::eq(args.rest()),
            };
            let game = self.modio.game(game_id);
            let mods = game.mods().list(&filter);
            let task = game
                .get()
                .join(mods)
                .and_then(move |(game, list)| {
                    let ret = match list.count {
                        0 => {
                            Some(channel.say("no mods found."))
                        }
                        1 => {
                            let mod_ = &list[0];
                            Some(channel.send_message(|m| mod_.create_message(&game, m)))
                        }
                        _ => {
                            let mods = list.into_iter().fold(ContentBuilder::default(), |mut buf, mod_| {
                                let _ = writeln!(&mut buf, "{}. {}", mod_.id, mod_.name);
                                buf
                            });
                            for content in mods {
                                let ret = channel.send_message(|m| {
                                    m.embed(|e| e.title("Matching mods").description(content))
                                });
                                if let Err(e) = ret {
                                    eprintln!("{:?}", e);
                                }
                            }
                            None
                        }
                    };
                    if let Some(Err(e)) = ret {
                        eprintln!("{:?}", e);
                    }
                    Ok(())
                })
                .map_err(|e| {
                    eprintln!("{}", e)
                });

            self.executor.spawn(task);
        }
    }

    options(opts) {
        opts.desc = Some("Search mods or show the details for a single mod.".to_string());
        opts.usage = Some("mod <id|search>".to_string());
        opts.guild_only = true;
        opts.bucket = Some("simple".to_string());
        opts.min_args = Some(1);
    }
);

command!(
    Popular(self, ctx, msg) {
        let channel = msg.channel_id;
        let game_id = msg.guild_id.and_then(|id| {
            Settings::game(ctx, id)
        });
        if let Some(id) = game_id {
            let filter = with_limit(10).order_by(PopularFilter::desc());
            let task = list_mods(
                self.modio.game(id).mods(),
                &filter,
                Some(10),
                channel,
            );

            self.executor.spawn(task);
        } else {
            let _ = channel.say("default game is not set.");
        }
    }

    options(opts) {
        opts.desc = Some("List popular mods.".to_string());
        opts.usage = Some("popular".to_string());
        opts.guild_only = true;
        opts.bucket = Some("simple".to_string());
        opts.max_args = Some(0);
    }
);

fn list_mods(
    mods: modio::mods::Mods,
    filter: &Filter,
    limit: Option<usize>,
    channel: ChannelId,
) -> impl Future<Item = (), Error = ()> + Send + 'static {
    let mut limit = limit;
    mods.iter(filter)
        .take_while(move |_| match limit.as_mut() {
            Some(ref v) if **v == 0 => Ok(false),
            Some(v) => {
                *v -= 1;
                Ok(true)
            }
            None => Ok(true),
        })
        .fold(ContentBuilder::default(), |mut buf, mod_| {
            let _ = buf.write_str(&format!("{}. {}\n", mod_.id, mod_.name));
            future::ok::<_, modio::Error>(buf)
        })
        .and_then(move |mods| {
            if mods.is_empty() {
                let ret = channel.say("no mods found.");
                if let Err(e) = ret {
                    eprintln!("{:?}", e);
                }
            } else {
                for content in mods {
                    let ret =
                        channel.send_message(|m| m.embed(|e| e.title("Mods").description(content)));
                    if let Err(e) = ret {
                        eprintln!("{:?}", e);
                    }
                }
            };
            Ok(())
        })
        .map_err(|e| {
            eprintln!("{}", e);
        })
}

pub trait ModExt {
    fn create_new_mod_message(&self, _: &Game, _: CreateMessage) -> CreateMessage;

    fn create_message(&self, _: &Game, _: CreateMessage) -> CreateMessage;

    fn create_fields(&self, is_new: bool) -> Vec<EmbedField>;
}

impl ModExt for Mod {
    fn create_fields(&self, is_new: bool) -> Vec<EmbedField> {
        fn ratings(stats: &Statistics) -> Option<EmbedField> {
            Some((
                "Ratings",
                format!(
                    r#"- Rank: {}/{}
- Downloads: {}
- Subscribers: {}
- Votes: +{}/-{}"#,
                    stats.popularity.rank_position,
                    stats.popularity.rank_total,
                    stats.downloads_total,
                    stats.subscribers_total,
                    stats.ratings.positive,
                    stats.ratings.negative,
                ),
                true,
            ))
        }
        fn dates(m: &Mod) -> Option<EmbedField> {
            let added = format_timestamp(m.date_added as i64);
            let updated = format_timestamp(m.date_updated as i64);
            Some((
                "Dates",
                format!("- Created: {}\n- Updated: {}", added, updated),
                true,
            ))
        }
        fn info(m: &Mod) -> Option<EmbedField> {
            let mut info = String::from("Links: ");
            if let Some(homepage) = &m.homepage_url {
                let _ = write!(info, "[Homepage]({}), ", homepage);
            }
            if let Some(f) = &m.modfile {
                let _ = writeln!(info, "[Download]({})", f.download.binary_url);
                if let Some(version) = &f.version {
                    let _ = writeln!(info, "Version: {}", version);
                }
                let _ = writeln!(info, "Size: {}", bytesize::to_string(f.filesize, false));
            }
            if info.len() > 7 {
                Some(("Info", info, true))
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
            Some(("Tags", tags, true))
        }

        let fields = if is_new {
            vec![info(self), tags(self)]
        } else {
            vec![ratings(&self.stats), info(self), dates(self), tags(self)]
        };
        fields.into_iter().flatten().collect()
    }

    fn create_message(&self, game: &Game, m: CreateMessage) -> CreateMessage {
        m.embed(|e| {
            e.title(self.name.to_string())
                .url(self.profile_url.to_string())
                .description(self.summary.to_string())
                .thumbnail(&self.logo.thumb_320x180)
                .author(|a| {
                    a.name(&game.name)
                        .icon_url(&game.icon.thumb_64x64.to_string())
                        .url(&game.profile_url.to_string())
                })
                .footer(|f| self.submitted_by.create_footer(f))
                .fields(self.create_fields(false))
        })
    }

    fn create_new_mod_message(&self, game: &Game, m: CreateMessage) -> CreateMessage {
        m.embed(|e| {
            e.title(&self.name)
                .url(&self.profile_url)
                .description(&self.summary)
                .image(&self.logo.thumb_640x360)
                .author(|a| {
                    a.name(&game.name)
                        .icon_url(&game.icon.thumb_64x64.to_string())
                        .url(&game.profile_url.to_string())
                })
                .footer(|f| self.submitted_by.create_footer(f))
                .fields(self.create_fields(true))
        })
    }
}
