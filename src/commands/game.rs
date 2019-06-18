use std::sync::mpsc;

use futures::future;
use modio::filter::prelude::*;

use crate::commands::prelude::*;
use crate::util::ContentBuilder;

type Stats = (usize, u32, u32);

#[command("games")]
#[description = "List all games on <https://mod.io>"]
#[bucket = "simple"]
#[max_args(0)]
pub fn list_games(ctx: &mut Context, msg: &Message) -> CommandResult {
    let channel = msg.channel_id;
    let data = ctx.data.read();
    let modio = data.get::<ModioKey>().expect("get modio failed");
    let exec = data.get::<ExecutorKey>().expect("get exec failed");

    let (tx, rx) = mpsc::channel();

    let task = modio
        .games()
        .iter(&Default::default())
        .fold(ContentBuilder::default(), |mut buf, game| {
            let _ = writeln!(&mut buf, "{}. {}", game.id, game.name);
            future::ok::<_, modio::Error>(buf)
        })
        .and_then(move |games| {
            tx.send(games).unwrap();
            Ok(())
        })
        .map_err(|e| {
            eprintln!("{}", e);
        });
    exec.spawn(task);

    let games = rx.recv().unwrap();
    for content in games {
        let ret =
            channel.send_message(&ctx, |m| m.embed(|e| e.title("Games").description(content)));
        if let Err(e) = ret {
            eprintln!("{:?}", e);
        }
    }
    Ok(())
}

#[command]
#[description = "Display or set the default game."]
#[usage = "game [id|search]"]
#[bucket = "simple"]
#[min_args(0)]
#[only_in(guilds)]
pub fn game(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let id = match args.single::<u32>() {
        Ok(id) => Some(Identifier::Id(id)),
        Err(ArgError::Parse(_)) => Some(Identifier::Search(args.rest().into())),
        Err(_) => None,
    };
    match id {
        Some(id) => set_game(ctx, msg, id),
        None => get_game(ctx, msg),
    }?;
    Ok(())
}

fn get_game(ctx: &mut Context, msg: &Message) -> CommandResult {
    let channel = msg.channel_id;
    let game_id = msg.guild_id.and_then(|id| Settings::game(ctx, id));

    let data = ctx.data.read();
    let modio = data.get::<ModioKey>().expect("get modio failed");
    let exec = data.get::<ExecutorKey>().expect("get exec failed");

    if let Some(id) = game_id {
        let (tx, rx) = mpsc::channel();
        let stats = modio
            .game(id)
            .mods()
            .statistics(&Default::default())
            .collect()
            .and_then(|list| {
                let total = list.len();
                Ok(list
                    .into_iter()
                    .fold((total, 0, 0), |(total, mut dl, mut sub), s| {
                        dl += s.downloads_total;
                        sub += s.subscribers_total;
                        (total, dl, sub)
                    }))
            });
        let task = modio
            .game(id)
            .get()
            .join(stats)
            .and_then(move |(game, stats)| {
                tx.send((game, stats)).unwrap();
                Ok(())
            })
            .map_err(|e| {
                eprintln!("{}", e);
            });
        exec.spawn(task);

        let (game, stats) = rx.recv().unwrap();
        if let Err(e) = channel.send_message(&ctx, |m| game.create_message(stats, m)) {
            eprintln!("{} {:?}", e, e);
        }
    }
    Ok(())
}

fn set_game(ctx: &mut Context, msg: &Message, id: Identifier) -> CommandResult {
    let channel = msg.channel_id;

    if let Some(guild_id) = msg.guild_id {
        let game = {
            let data = ctx.data.read();
            let modio = data.get::<ModioKey>().expect("get modio failed");
            let exec = data.get::<ExecutorKey>().expect("get exec failed");
            let (tx, rx) = mpsc::channel();
            let filter = match id {
                Identifier::Id(id) => Id::eq(id),
                Identifier::Search(id) => Fulltext::eq(id),
            };
            let task = modio
                .games()
                .list(&filter)
                .and_then(|mut list| Ok(list.shift()))
                .and_then(move |game| {
                    tx.send(game).unwrap();
                    Ok(())
                })
                .map_err(|e| {
                    eprintln!("{}", e);
                });
            exec.spawn(task);

            rx.recv().unwrap()
        };

        if let Some(game) = game {
            let mut ctx2 = ctx.clone();
            Settings::set_game(&mut ctx2, guild_id, game.id);
            let _ = channel.say(&ctx, format!("Game is set to '{}'", game.name));
        } else {
            let _ = channel.say(&ctx, "Game not found");
        }
    }
    Ok(())
}

trait GameExt {
    fn create_fields(&self, _: Stats) -> Vec<EmbedField>;

    fn create_message<'a, 'b>(
        &self,
        _: Stats,
        m: &'b mut CreateMessage<'a>,
    ) -> &'b mut CreateMessage<'a>;
}

impl GameExt for modio::games::Game {
    fn create_fields(&self, s: Stats) -> Vec<EmbedField> {
        fn info(g: &modio::games::Game) -> EmbedField {
            (
                "Info",
                format!(
                    r#"**Id:** {}
**Name-Id:** {}
**Profile:** {}"#,
                    g.id,
                    g.name_id,
                    g.profile_url.to_string(),
                ),
                true,
            )
        }
        fn stats(stats: Stats) -> EmbedField {
            let (total, downloads, subs) = stats;
            (
                "Stats",
                format!(
                    r#"**Mods:** {}
**Subscribers:** {}
**Downloads:** {}"#,
                    total, subs, downloads,
                ),
                true,
            )
        }
        vec![info(self), stats(s)]
    }

    fn create_message<'a, 'b>(
        &self,
        stats: Stats,
        m: &'b mut CreateMessage<'a>,
    ) -> &'b mut CreateMessage<'a> {
        m.embed(|e| {
            e.title(self.name.to_string())
                .url(self.profile_url.to_string())
                // TODO fix UserExt trait
                // .author(|a| self.submitted_by.create_author(a))
                .description(self.summary.to_string())
                .image(self.logo.thumb_640x360.to_string())
                .fields(self.create_fields(stats))
        });
        m
    }
}
