use either::Either;
use modio::mods::{Mod, ModsListOptions, Statistics};

use crate::commands::prelude::*;

command!(
    ListMods(self, ctx, msg) {
        let channel = msg.channel_id;
        let game_id = msg.guild_id.and_then(|id| {
            let data = ctx.data.lock();
            let map = data.get::<GameKey>().expect("failed to get map");
            map.get(&id).cloned()
        });
        if let Some(Identifier::Id(id)) = game_id {
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
        opts.max_args = Some(0);
    }
);

command!(
    ModInfo(self, ctx, msg, args) {
        let channel = msg.channel_id;
        let game_id = msg.guild_id.and_then(|id| {
            let data = ctx.data.lock();
            let map = data.get::<GameKey>().expect("failed to get map");
            map.get(&id).cloned()
        });
        if let Some(Identifier::Id(game_id)) = game_id {
            let mut opts = ModsListOptions::new();
            match args.single::<u32>() {
                Ok(id) => opts.id(Operator::Equals, id),
                Err(_) => opts.fulltext(args.rest()),
            };
            let task = self
                .modio
                .game(game_id)
                .mods()
                .list(&opts)
                .and_then(move |list| {
                    let ret = match list.count {
                        0 => {
                            channel.say("no mods found.")
                        }
                        1 => {
                            let mod_ = &list[0];
                            channel.send_message(|m| mod_.create_message(m))
                        }
                        _ => {
                            channel.send_message(|m| list.create_message(m))
                        }
                    };
                    if let Err(e) = ret {
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
        opts.min_args = Some(1);
    }
);

command!(
    Popular(self, ctx, msg) {
        let channel = msg.channel_id;
        let game_id = msg.guild_id.and_then(|id| {
            let data = ctx.data.lock();
            let map = data.get::<GameKey>().expect("failed to get map");
            map.get(&id).cloned()
        });
        if let Some(Identifier::Id(id)) = game_id {
            let mut opts = ModsListOptions::new();
            opts.limit(10);
            opts.sort_by(ModsListOptions::POPULAR, modio::filter::Order::Desc);
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
        opts.desc = Some("List popular mods.".to_string());
        opts.usage = Some("popular".to_string());
        opts.guild_only = true;
        opts.max_args = Some(0);
    }
);

fn list_mods<C>(
    mods: modio::mods::Mods<C>,
    opts: &ModsListOptions,
    channel: ChannelId,
) -> impl Future<Item = (), Error = ()> + Send + 'static
where
    C: Clone + Connect + 'static,
{
    mods.list(opts)
        .and_then(move |list| {
            let ret = if list.count == 0 {
                channel.say("no mods found.")
            } else {
                channel.send_message(|m| list.create_message(m))
            };

            if let Err(e) = ret {
                eprintln!("{:?}", e);
            }
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
                .map(|t| t.to_string())
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
                .thumbnail(self.logo.thumb_640x360.to_string())
                .fields(self.create_fields())
        })
    }
}

impl ModioListResponseExt for ModioListResponse<Mod> {
    fn create_message(&self, m: CreateMessage) -> CreateMessage {
        let mut buf = String::new();
        for m in &self.data {
            let _ = writeln!(&mut buf, "{}. {}", m.id, m.name);
        }
        m.embed(|e| e.description(buf))
    }
}
