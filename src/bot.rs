use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use modio::Modio;
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::cluster::Events;
use twilight_gateway::{Cluster, EventTypeFlags, Intents};
use twilight_http::client::InteractionClient;
use twilight_http::Client;
use twilight_model::application::interaction::InteractionData;
use twilight_model::gateway::event::Event;
use twilight_model::gateway::payload::outgoing::update_presence::UpdatePresencePayload;
use twilight_model::gateway::presence::{ActivityType, MinimalActivity, Status};
use twilight_model::oauth::Application;

use crate::commands;
use crate::config::Config;
use crate::db::{load_settings, DbPool, Settings, Subscriptions};
use crate::error::Error;
use crate::metrics::Metrics;

#[derive(Clone)]
pub struct Context {
    pub application: Application,
    pub client: Arc<Client>,
    pub cache: Arc<InMemoryCache>,
    pub modio: Modio,
    pub pool: DbPool,
    pub settings: Arc<Mutex<Settings>>,
    pub subscriptions: Subscriptions,
    pub metrics: Metrics,
}

impl Context {
    pub fn interaction(&self) -> InteractionClient<'_> {
        self.client.interaction(self.application.id)
    }
}

pub async fn initialize(
    config: &Config,
    modio: Modio,
    pool: DbPool,
    metrics: Metrics,
) -> Result<(Cluster, Events, Context), Error> {
    let client = Arc::new(Client::new(config.bot.token.clone()));
    let application = client.current_user_application().await?.model().await?;

    let interaction = client.interaction(application.id);
    commands::register(&interaction).await?;

    let presence = UpdatePresencePayload::new(
        [MinimalActivity {
            kind: ActivityType::Playing,
            name: "/help".into(),
            url: None,
        }
        .into()],
        false,
        None,
        Status::Online,
    )
    .expect("required activity is provided");

    let (cluster, events) = Cluster::builder(config.bot.token.clone(), Intents::GUILDS)
        .event_types(
            EventTypeFlags::READY
                | EventTypeFlags::GUILD_CREATE
                | EventTypeFlags::GUILD_DELETE
                | EventTypeFlags::INTERACTION_CREATE,
        )
        .presence(presence)
        .http_client(Arc::clone(&client))
        .build()
        .await?;

    let cache = InMemoryCache::builder()
        .resource_types(ResourceType::USER_CURRENT)
        .build();

    let ctx = Context {
        application,
        client,
        cache: Arc::new(cache),
        modio,
        pool: pool.clone(),
        settings: Arc::new(Mutex::new(Settings {
            pool: pool.clone(),
            data: HashMap::new(),
        })),
        subscriptions: Subscriptions { pool },
        metrics,
    };

    Ok((cluster, events, ctx))
}

pub async fn handle_event(event: Event, context: Context) {
    context.cache.update(&event);

    match event {
        Event::Ready(ready) => {
            let guilds = ready.guilds.iter().map(|g| g.id.get()).collect::<Vec<_>>();
            tracing::info!("Guilds: {guilds:?}");
            context.metrics.guilds.set(ready.guilds.len() as u64);

            if let Err(e) = context.subscriptions.cleanup(&guilds) {
                tracing::error!("{e}");
            }
            let guilds = ready
                .guilds
                .into_iter()
                .map(|g| g.id.get())
                .collect::<Vec<_>>();
            let data = load_settings(&context.pool, &guilds).unwrap_or_default();
            tracing::info!("{data:?}");

            let mut settings = context.settings.lock().unwrap();
            settings.data.extend(data);
        }
        Event::InteractionCreate(interaction) => match &interaction.data {
            Some(InteractionData::ApplicationCommand(command)) => {
                commands::handle_command(&context, &interaction, command).await;
            }
            Some(InteractionData::MessageComponent(component)) => {
                commands::handle_component(&context, &interaction, component).await;
            }
            _ => {}
        },
        _ => {}
    }
}
