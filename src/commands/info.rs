use either::Either;
use futures::Future;
use modio::filter::Operator;
use modio::mods::{Mod, ModsListOptions, Statistics};
use modio::users::User;
use serenity::builder::{CreateEmbedAuthor, CreateMessage};

use crate::util::{format_timestamp, GameKey, Identifier};

command!(
    ModInfo(self, ctx, msg, args) {
        let channel = msg.channel_id;
        let game_id = msg.guild_id.and_then(|id| {
            let data = ctx.data.lock();
            let map = data.get::<GameKey>().expect("failed to get map");
            map.get(&id).cloned()
        });
        if let Some(Identifier::Id(game_id)) = game_id {
            if let Ok(id) = args.single::<Identifier>() {
                let mut opts = ModsListOptions::new();
                match id {
                    Identifier::Id(id) => opts.id(Operator::Equals, id),
                    Identifier::NameId(id) => opts.name_id(Operator::Equals, id),
                };
                let task = self
                    .modio
                    .game(game_id)
                    .mods()
                    .list(&opts)
                    .and_then(|mut list| Ok(list.shift()))
                    .and_then(move |m| {
                        if let Some(mod_) = m {
                            let r = channel.send_message(|m| {
                                mod_.create_message(m)
                            });
                            if let Err(e) = r {
                                eprintln!("{:?}", e);
                            }
                        }
                        Ok(())
                    })
                    .map_err(|e| {
                        eprintln!("{}", e)
                    });

                self.executor.spawn(task);
            }
        }
    }

    options(opts) {
        opts.desc = Some("Show mod details".to_string());
        opts.usage = Some("mod id|name-id".to_string());
        opts.guild_only = true;
        opts.min_args = Some(1);
        opts.max_args = Some(1);
    }
);

trait UserExt {
    fn create_author(&self, _: CreateEmbedAuthor) -> CreateEmbedAuthor;
}

impl UserExt for User {
    fn create_author(&self, mut a: CreateEmbedAuthor) -> CreateEmbedAuthor {
        a = a.name(&self.username).url(&self.profile_url.to_string());
        if let Some(avatar) = &self.avatar {
            let icon = avatar.original.to_string();
            a = a.icon_url(&icon);
        }
        a
    }
}

trait ModExt {
    fn create_message(&self, _: CreateMessage) -> CreateMessage;

    fn create_fields(&self) -> Vec<(&str, String, bool)>;
}

impl ModExt for Mod {
    fn create_fields(&self) -> Vec<(&str, String, bool)> {
        fn ratings(stats: &Statistics) -> (&str, String, bool) {
            (
                "Ratings",
                format!(
                    r#"- Rank: {}/{}
- Downloads: {}
- Subscribers: {}
- Votes: {}/{}"#,
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
        fn dates(m: &Mod) -> (&str, String, bool) {
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
        fn info(m: &Mod) -> (&str, String, bool) {
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
        fn tags(m: &Mod) -> (&str, String, bool) {
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
