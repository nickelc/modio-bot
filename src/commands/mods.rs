use either::Either;
use futures::future;
use modio::filter::prelude::*;
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
            let task = self
                .modio
                .game(game_id)
                .mods()
                .list(&filter)
                .and_then(move |list| {
                    let ret = match list.count {
                        0 => {
                            Some(channel.say("no mods found."))
                        }
                        1 => {
                            let mod_ = &list[0];
                            Some(channel.send_message(|m| mod_.create_message(m)))
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
    channel: ChannelId,
) -> impl Future<Item = (), Error = ()> + Send + 'static {
    mods.iter(filter)
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

trait ModExt {
    fn create_message(&self, _: CreateMessage) -> CreateMessage;

    fn create_fields(&self) -> Vec<EmbedField>;
}

impl ModExt for Mod {
    fn create_fields(&self) -> Vec<EmbedField> {
        fn ratings(stats: &Statistics) -> EmbedField {
            (
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
            )
        }
        fn dates(m: &Mod) -> EmbedField {
            let added = format_timestamp(m.date_added as i64);
            let updated = format_timestamp(m.date_updated as i64);
            (
                "Dates",
                format!(
                    r#"- Created: {}
- Updated: {}"#,
                    added, updated,
                ),
                true,
            )
        }
        fn info(m: &Mod) -> EmbedField {
            let homepage = if let Some(homepage) = &m.homepage_url {
                Either::Left(format!("\nHomepage: {}\n", homepage))
            } else {
                Either::Right("")
            };
            let download = if let Some(f) = &m.modfile {
                Either::Left(format!("[{}]({})", f.filename, f.download.binary_url))
            } else {
                Either::Right("No file available")
            };
            (
                "Info",
                format!(
                    r#"Id: {}
Name-Id: {}{}
Download: {}"#,
                    m.id, m.name_id, homepage, download,
                ),
                true,
            )
        }
        fn tags(m: &Mod) -> EmbedField {
            let tags = m
                .tags
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            ("Tags", tags, false)
        }
        vec![ratings(&self.stats), info(self), dates(self), tags(self)]
    }

    fn create_message(&self, m: CreateMessage) -> CreateMessage {
        m.embed(|e| {
            e.title(self.name.to_string())
                .url(self.profile_url.to_string())
                .author(|a| self.submitted_by.create_author(a))
                .description(self.summary.to_string())
                .thumbnail(&self.logo.thumb_320x180)
                .fields(self.create_fields())
        })
    }
}
