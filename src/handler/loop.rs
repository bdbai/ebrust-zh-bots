use std::{pin::pin, time::Duration};

use compio::time::sleep;
use futures::future::{select, Either};
use telegram_types::bot::{
    methods::GetUpdates,
    types::{Message, Update, UpdateContent, UpdateId},
};
use tracing::{debug, error};

use crate::controller::IController;

use super::error::HandlerResult;

pub async fn run_loop<C: IController>(mut handler: super::Handler<'_, C>)
where
    C::EvalProcessingImpl: 'static,
{
    let mut update_offset = None;
    while let Err(e) = run_loop_inner(&mut handler, &mut update_offset).await {
        error!(?e, "loop error");
        sleep(Duration::from_secs(5)).await;
    }
}
async fn run_loop_inner<'e, C: IController>(
    handler: &mut super::Handler<'e, C>,
    update_offset: &mut Option<UpdateId>,
) -> HandlerResult<()>
where
    C::EvalProcessingImpl: 'static,
{
    let mut cancel_listener = handler.cancel_event.listen();
    loop {
        let updates = {
            let get_updates_req = GetUpdates {
                offset: *update_offset,
                timeout: Some(30),
                ..Default::default()
            };
            let get_updates_fut = pin!(handler
                .client
                .call_method_with_param::<_, Vec<Update>>("getUpdates", &get_updates_req,));
            match select(get_updates_fut, &mut cancel_listener).await {
                Either::Left((res, _)) => res?.into_result()?,
                Either::Right(_) => return Ok(()),
            }
        };
        debug!(?updates, "getUpdates");
        for update in updates {
            let next_offset = update.update_id + 1;
            *update_offset = update_offset
                .map(|o| o.max(next_offset))
                .or(Some(next_offset));
            let Some(content) = update.content else {
                continue;
            };
            let msg_id;
            let chat_id;
            let text;
            let msg_from;
            match content {
                UpdateContent::Message(Message {
                    message_id,
                    chat,
                    text: msg_text,
                    from: Some(from),
                    ..
                }) => {
                    msg_id = message_id;
                    chat_id = chat.id;
                    text = msg_text.unwrap_or_default();
                    msg_from = from;
                    debug!(?chat, ?msg_id, ?text, "message");
                }
                UpdateContent::EditedMessage(Message {
                    message_id,
                    chat,
                    text: msg_text,
                    from: Some(from),
                    ..
                }) => {
                    msg_id = message_id;
                    chat_id = chat.id;
                    text = msg_text.unwrap_or_default();
                    msg_from = from;
                    debug!(?chat, ?msg_id, ?text, "edited message");
                }
                UpdateContent::CallbackQuery(query) => {
                    if let Err(e) = handler.handle_message_callback_query(query).await {
                        error!(?e, "handle_message_callback_query error");
                    }
                    continue;
                }
                _ => continue,
            };
            let text = text.trim();
            if text.is_empty() {
                continue;
            }
            if let Some(command) = text.strip_prefix("/bval") {
                if let Err(e) = handler
                    .handle_new_message(chat_id, msg_id, msg_from.id, command.trim_start())
                    .await
                {
                    error!(?e, "handle_new_message error");
                }
            }
        }
    }
}
