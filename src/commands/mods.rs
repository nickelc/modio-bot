use std::sync::mpsc;

use futures::{future, StreamExt, TryStreamExt};

use modio::filter::prelude::*;
use modio::games::Game;
use modio::mods::filters::Popular as PopularFilter;
use modio::mods::{Mod, Statistics};

use crate::commands::prelude::*;
use crate::util::ContentBuilder;

#[command("mods")]
#[description = "List mods of the default game"]
#[only_in(guilds)]
#[bucket = "simple"]
#[max_args(0)]
pub fn list_mods(ctx: &mut Context, msg: &Message) -> CommandResult {
    let channel = msg.channel_id;
    let game_id = msg.guild_id.and_then(|id| Settings::game(ctx, id));

    let item = |m: &Mod| format!("{}. {}\n", m.id, m.name);

    if let Some(id) = game_id {
        let data = ctx.data.read();
        let modio = data.get::<ModioKey>().expect("get modio failed");
        let exec = data.get::<ExecutorKey>().expect("get exec failed");
        let (tx, rx) = mpsc::channel();

        let filter = Default::default();
        let game = modio.game(id);
        let mods = game.mods();

        let task = find_mods(game, mods, filter, None);

        exec.spawn(async move {
            match task.await {
                Ok(data) => tx.send(data).unwrap(),
                Err(e) => eprintln!("{}", e),
            }
        });

        let (game, mods) = rx.recv().unwrap();
        send_mods(&mut ctx.clone(), channel, game, mods, "Mods", item);
    } else {
        let _ = channel.say(&ctx, "default game is not set.");
    }
    Ok(())
}

#[command("mod")]
#[description = "Search mods or show the details for a single mod."]
#[usage = "mod <id|search>"]
#[only_in(guilds)]
#[bucket = "simple"]
#[min_args(1)]
pub fn mod_info(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let channel = msg.channel_id;
    let game_id = msg.guild_id.and_then(|id| Settings::game(ctx, id));

    if let Some(game_id) = game_id {
        let data = ctx.data.read();
        let modio = data.get::<ModioKey>().expect("get modio failed");
        let exec = data.get::<ExecutorKey>().expect("get exec failed");
        let (tx, rx) = mpsc::channel();

        let filter = match args.single::<u32>() {
            Ok(id) => Id::eq(id),
            Err(_) => Fulltext::eq(args.rest()),
        };

        let game = modio.game(game_id);
        let mods = game.mods().search(filter).first();
        let task = future::try_join(game.get(), mods);

        exec.spawn(async move {
            match task.await {
                Ok(data) => tx.send(data).unwrap(),
                Err(e) => eprintln!("{}", e),
            }
        });

        let (game, list) = rx.recv().unwrap();
        let ret = match list.len() {
            0 => Some(channel.say(&ctx, "no mods found.")),
            1 => {
                let mod_ = &list[0];
                Some(channel.send_message(&ctx, |m| mod_.create_message(&game, m)))
            }
            _ => {
                let mods = list
                    .into_iter()
                    .fold(ContentBuilder::default(), |mut buf, mod_| {
                        let _ = writeln!(&mut buf, "{}. {}", mod_.id, mod_.name);
                        buf
                    });
                for content in mods {
                    let ret = channel.send_message(&ctx, |m| {
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
    }
    Ok(())
}

#[command]
#[description = "List popular mods."]
#[only_in(guilds)]
#[bucket = "simple"]
#[max_args(0)]
pub fn popular(ctx: &mut Context, msg: &Message) -> CommandResult {
    let channel = msg.channel_id;
    let game_id = msg.guild_id.and_then(|id| Settings::game(ctx, id));

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
        let data = ctx.data.read();
        let modio = data.get::<ModioKey>().expect("get modio failed");
        let exec = data.get::<ExecutorKey>().expect("get exec failed");
        let (tx, rx) = mpsc::channel();

        let filter = with_limit(10).order_by(PopularFilter::desc());
        let game = modio.game(id);
        let mods = game.mods();
        let task = find_mods(game, mods, filter, Some(10));

        exec.spawn(async move {
            match task.await {
                Ok(data) => tx.send(data).unwrap(),
                Err(e) => eprintln!("{}", e),
            }
        });

        let (game, mods) = rx.recv().unwrap();
        send_mods(&mut ctx.clone(), channel, game, mods, "Popular Mods", item);
    } else {
        let _ = channel.say(&ctx, "default game is not set.");
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
    let mods = mods
        .search(filter)
        .iter()
        .take_while(move |_| match limit.as_mut() {
            Some(ref v) if **v == 0 => future::ready(false),
            Some(v) => {
                *v -= 1;
                future::ready(true)
            }
            None => future::ready(true),
        })
        .try_collect();

    future::try_join(game.get(), mods)
}

fn send_mods<F>(
    ctx: &mut Context,
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
            let ret = channel.send_message(&ctx, |m| {
                m.embed(|e| {
                    e.title(title).description(content).author(|a| {
                        a.name(&game.name)
                            .icon_url(&game.icon.thumb_64x64.to_string())
                            .url(&game.profile_url.to_string())
                    })
                })
            });
            if let Err(e) = ret {
                eprintln!("{:?}", e);
            }
        }
    } else {
        let ret = channel.say(&ctx, "no mods found.");
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

    fn create_message<'a, 'b>(
        &self,
        game: &Game,
        m: &'b mut CreateMessage<'a>,
    ) -> &'b mut CreateMessage<'a> {
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

    fn create_new_mod_message<'a, 'b>(
        &self,
        game: &Game,
        m: &'b mut CreateMessage<'a>,
    ) -> &'b mut CreateMessage<'a> {
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
