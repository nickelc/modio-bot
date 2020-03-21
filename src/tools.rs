use std::env;

use crate::config::Config;

pub async fn tools(config: &Config) -> bool {
    let mut args = env::args().skip(1);

    let mut command = match args.next() {
        Some(c) => c,
        None => return false,
    };

    command = command.trim().to_lowercase();
    let command = command.as_str();

    match command {
        "print-servers" => print::print_servers(config).await,
        _ => return false,
    };

    true
}

mod print {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use serenity::async_trait;
    use serenity::model::prelude::*;
    use serenity::prelude::*;
    use tokio::time;

    use crate::config::Config;

    #[derive(Default)]
    struct GuildCounter {
        init: bool,
        total: usize,
        ready: usize,
    }

    impl TypeMapKey for GuildCounter {
        type Value = Arc<Mutex<GuildCounter>>;
    }

    impl GuildCounter {
        fn set_total(&mut self, total: usize) {
            self.init = true;
            self.total = total;
        }

        fn add_ready(&mut self) {
            self.ready += 1;
        }

        fn all_ready(&self) -> bool {
            self.init && self.ready >= self.total
        }
    }

    struct Handler;

    #[async_trait]
    impl EventHandler for Handler {
        async fn ready(&self, ctx: Context, ready: Ready) {
            let guilds = ready.guilds.len();
            let mut data = ctx.data.write().await;
            let mut counter = data
                .get_mut::<GuildCounter>()
                .expect("failed to get GuildCounter")
                .lock()
                .expect("failed to lock GuildCounter");
            counter.set_total(guilds);
            println!("{} servers:", guilds);
        }

        async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: bool) {
            let mut data = ctx.data.write().await;
            let mut counter = data
                .get_mut::<GuildCounter>()
                .expect("failed to get GuildCounter")
                .lock()
                .expect("failed to lock GuildCounter");
            counter.add_ready();

            println!(
                " - {} (id: {}, members: {})",
                guild.name,
                guild.id,
                guild.members.len(),
            );
        }
    }

    pub async fn print_servers(config: &Config) {
        let counter = Arc::new(Mutex::new(GuildCounter::default()));

        let thread_counter = counter.clone();

        let mut client = Client::builder(&config.bot.token)
            .event_handler(Handler)
            .await
            .expect("failed to create client");
        {
            let mut data = client.data.write().await;
            data.insert::<GuildCounter>(thread_counter);
        }
        tokio::spawn(async move {
            client.start().await.expect("failed to start client");
        });

        loop {
            let done = {
                counter
                    .lock()
                    .expect("failed to locl GuildCounter")
                    .all_ready()
            };

            if done {
                break;
            }
            time::delay_for(Duration::from_millis(200)).await;
        }
        println!("done");
        std::process::exit(0);
    }
}
