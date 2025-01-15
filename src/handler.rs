mod callback_query;
mod error;
mod r#loop;
mod new_message;
mod render;
mod tg_client;

use std::sync::Arc;

use event_listener::Event;
pub(crate) use r#loop::run_loop;
pub(crate) use tg_client::{TgClient, TgEnv};

pub struct Handler<'e, C> {
    pub client: Arc<TgClient>,
    pub controller: C,
    pub cancel_event: &'e Event,
}
