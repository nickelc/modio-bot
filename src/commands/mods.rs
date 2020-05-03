use std::borrow::Cow;
use std::time::Duration;

use modio::filter::prelude::*;
use modio::games::{ApiAccessOptions, Game};
use modio::mods::filters::Popular as PopularFilter;
use modio::mods::{Mod, Statistics};

use crate::commands::prelude::*;
use crate::util::ContentBuilder;

#[command("mod")]
#[description = "Search mods or show the details for a single mod."]
#[usage = "mod <id|search>"]
#[only_in(guilds)]
#[bucket = "simple"]
#[min_args(1)]
pub async fn mod_info(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    list_mods(ctx, msg, args).await
}

#[command("mods")]
#[description = "List mods of the default game"]
#[usage = "mods [id|search]"]
#[only_in(guilds)]
#[bucket = "simple"]
pub async fn list_mods(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let channel = msg.channel_id;
    let game_id = {
        let data = ctx.data.read().await;
        let settings = data.get::<Settings>().expect("get settings failed");
        msg.guild_id.and_then(|id| settings.game(id))
    };

    if let Some(id) = game_id {
        let data = ctx.data.read().await;
        let modio = data.get::<ModioKey>().expect("get modio failed");

        let (filter, title): (Filter, Cow<'_, _>) = if args.is_empty() {
            (Filter::default(), "Mods".into())
        } else {
            match args.single::<u32>() {
                Ok(id) => (Id::eq(id), "Mods".into()),
                Err(_) => (
                    Fulltext::eq(args.rest()),
                    format!("Mods matching: '{}'", args.rest()).into(),
                ),
            }
        };
        let game = modio.game(id);
        let mods = game.mods();

        let mut first = true;
        let mut st = mods.search(filter.and(with_limit(20))).paged().await?;
        loop {
            match st.try_next().await? {
                None if first => {
                    channel.say(ctx, "no mods found.").await?;
                    break;
                }
                None => {
                    channel.say(ctx, "no other mods found.").await?;
                    break;
                }
                Some(list) if list.len() == 1 && first => {
                    let game = game.get().await?;
                    let mod_ = &list[0];
                    channel
                        .send_message(ctx, |m| mod_.create_message(&game, m))
                        .await?;
                    break;
                }
                Some(list) => {
                    let content = list.iter().try_fold(String::new(), |mut buf, mod_| {
                        writeln!(&mut buf, "{}. {}", mod_.id, mod_.name)?;
                        Ok::<_, std::fmt::Error>(buf)
                    })?;
                    channel
                        .send_message(ctx, |m| {
                            m.embed(|e| {
                                e.title(&title)
                                    .description(content)
                                    .footer(|f| f.text("Type `next` within 15s for the next page"))
                            })
                        })
                        .await?;
                }
            }
            first = false;

            let collector = msg
                .author
                .await_reply(ctx)
                .channel_id(channel)
                .filter(|m| m.content.to_lowercase() == "next")
                .timeout(Duration::from_secs(15));

            if collector.await.is_none() {
                break;
            }
        }
    }
    Ok(())
}

#[command]
#[description = "List popular mods."]
#[only_in(guilds)]
#[bucket = "simple"]
#[max_args(0)]
pub async fn popular(ctx: &Context, msg: &Message) -> CommandResult {
    let channel = msg.channel_id;
    let game_id = {
        let data = ctx.data.read().await;
        let settings = data.get::<Settings>().expect("get settings failed");
        msg.guild_id.and_then(|id| settings.game(id))
    };

    let item = |m: &Mod| {
        format!(
            "{:02}. [{}]({}) ({}) +{}/-{}\n",
            m.stats.popularity.rank_position,
            m.name,
            m.profile_url,
            m.id,
            m.stats.ratings.positive,
            m.stats.ratings.negative,
        )
    };

    if let Some(id) = game_id {
        let data = ctx.data.read().await;
        let modio = data.get::<ModioKey>().expect("get modio failed");

        let filter = with_limit(10).order_by(PopularFilter::desc());
        let game = modio.game(id);
        let mods = game.mods();
        let (game, mods) = find_mods(game, mods, filter, Some(10)).await?;

        send_mods(ctx, channel, game, mods, "Popular Mods", item).await;
    } else {
        let _ = channel.say(ctx, "default game is not set.").await;
    }
    Ok(())
}

fn find_mods(
    game: modio::games::GameRef,
    mods: modio::mods::Mods,
    filter: Filter,
    limit: Option<usize>,
) -> impl Future<Output = Result<(Game, Vec<Mod>), modio::Error>> {
    let mut limit = limit;
    let mods = mods.search(filter).iter().and_then(move |iter| {
        iter.take_while(move |_| match limit.as_mut() {
            Some(ref v) if **v == 0 => future::ready(false),
            Some(v) => {
                *v -= 1;
                future::ready(true)
            }
            None => future::ready(true),
        })
        .try_collect()
    });

    future::try_join(game.get(), mods)
}

async fn send_mods<F>(
    ctx: &Context,
    channel: ChannelId,
    game: Game,
    mods: Vec<Mod>,
    title: &'static str,
    item: F,
) where
    F: Fn(&Mod) -> String + Send + 'static,
{
    if !mods.is_empty() {
        let mods = mods
            .iter()
            .fold(ContentBuilder::default(), |mut buf, mod_| {
                let _ = buf.write_str(&item(&mod_));
                buf
            });
        for content in mods {
            let ret = channel
                .send_message(ctx, |m| {
                    m.embed(|e| {
                        e.title(title).description(content).author(|a| {
                            a.name(&game.name)
                                .icon_url(&game.icon.thumb_64x64.to_string())
                                .url(&game.profile_url.to_string())
                        })
                    })
                })
                .await;
            if let Err(e) = ret {
                eprintln!("{:?}", e);
            }
        }
    } else {
        let ret = channel.say(ctx, "no mods found.").await;
        if let Err(e) = ret {
            eprintln!("{:?}", e);
        }
    }
}

pub trait ModExt {
    fn create_new_mod_message<'a, 'b>(
        &self,
        _: &Game,
        _: &'b mut CreateMessage<'a>,
    ) -> &'b mut CreateMessage<'a>;

    fn create_message<'a, 'b>(
        &self,
        _: &Game,
        _: &'b mut CreateMessage<'a>,
    ) -> &'b mut CreateMessage<'a>;

    fn create_fields(&self, is_new: bool, with_ddl: bool) -> Vec<EmbedField>;
}

impl ModExt for Mod {
    fn create_fields(&self, is_new: bool, with_ddl: bool) -> Vec<EmbedField> {
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
            vec![info(self, with_ddl), tags(self)]
        } else {
            vec![
                ratings(&self.stats),
                info(self, with_ddl),
                dates(self),
                tags(self),
            ]
        };
        fields.into_iter().flatten().collect()
    }

    fn create_message<'a, 'b>(
        &self,
        game: &Game,
        m: &'b mut CreateMessage<'a>,
    ) -> &'b mut CreateMessage<'a> {
        let with_ddl = game
            .api_access_options
            .contains(ApiAccessOptions::ALLOW_DIRECT_DOWNLOAD);

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
                .fields(self.create_fields(false, with_ddl))
        })
    }

    fn create_new_mod_message<'a, 'b>(
        &self,
        game: &Game,
        m: &'b mut CreateMessage<'a>,
    ) -> &'b mut CreateMessage<'a> {
        let with_ddl = game
            .api_access_options
            .contains(ApiAccessOptions::ALLOW_DIRECT_DOWNLOAD);

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
                .fields(self.create_fields(true, with_ddl))
        })
    }
}
