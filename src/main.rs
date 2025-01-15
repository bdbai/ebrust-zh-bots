use std::{env, sync::Arc};

use event_listener::Event;
use telegram_types::bot::types::User;
use tracing::{debug, info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

mod controller;
mod handler;
mod repository;
mod service;

use controller::Controller;
use handler::{run_loop, TgClient, TgEnv};

#[compio::main]
async fn main() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy()
        .add_directive("h2=warn".parse().unwrap());
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let Ok(tg_api_key) = env::var("EBRZ_TG_API_KEY") else {
        panic!("invalid EBRZ_TG_API_KEY");
    };
    let tg_server_url = match env::var("EBRZ_TG_ENV").as_deref() {
        Ok("test") => TgEnv::Test,
        Ok("prod") => TgEnv::Prod,
        _ => panic!("invalid EBRZ_TG_ENV"),
    };

    let db = rusqlite::Connection::open("ebrz.db").expect("Cannot open database");

    let controller = Controller::new(
        repository::init_db(db).expect("Cannot initialize repository"),
        service::playground::PlaygroundService::new("https://play.rust-lang.org".into()),
    );

    info!("getting bot info");
    let client = Arc::new(TgClient::new(tg_api_key, tg_server_url));
    let me = client.call_method::<User>("getMe").await.unwrap();
    debug!(?me, "bot getMe");
    // TODO: warn when inline not enabled
    let cancel_event = Event::new();
    let handler = handler::Handler {
        client,
        controller,
        cancel_event: &cancel_event,
    };

    // TODO: graceful shutdown
    run_loop(handler).await;
}
