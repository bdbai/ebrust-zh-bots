use std::sync::Arc;

use compio::runtime::spawn;
use event_listener::EventListener;
use telegram_types::bot::{
    methods::{ChatTarget, EditMessageText, SendMessage},
    types::{ChatId, Message, MessageId, ParseMode, UserId},
};
use tracing::error;

use crate::controller::{
    EvalResponse, EvalResultResponse, IController, UpdateEvalMsgId, WaitForEvalResult,
};

use super::{error::HandlerResult, render::render_page_data, Handler, TgClient};

const PROCESSING_MESSAGE_TEXT: &str = "<i>Processing...</i>";

impl<'e, C: IController> Handler<'e, C>
where
    C::EvalProcessingImpl: 'static,
{
    pub(super) async fn handle_new_message(
        &self,
        chat_id: ChatId,
        user_msg_id: MessageId,
        user_id: UserId,
        command: &str,
    ) -> HandlerResult<()> {
        let command = command.to_owned();
        let res = self
            .controller
            .new_eval(chat_id.0, user_msg_id.0, user_id.0, command.clone())
            .await;
        let chat_target = ChatTarget::Id(chat_id);
        let mut processing = match res {
            EvalResponse::Processing(p) => p,
            EvalResponse::Err(e) => {
                self.client
                    .call_method_with_param::<_, Message>(
                        "sendMessage",
                        &SendMessage::new(
                            chat_target.clone(),
                            format!("<i>Fatal error: {}</i>", htmlize::escape_text(e)),
                        )
                        .parse_mode(ParseMode::HTML)
                        .reply(user_msg_id),
                    )
                    .await?;
                return Ok(());
            }
        };
        let eval_msg_id = loop {
            match processing.eval_msg_id {
                Some(eval_msg_id) => {
                    let edit_res = self
                        .client
                        .call_method_with_param::<_, Message>(
                            "editMessageText",
                            &EditMessageText::new(
                                chat_target.clone(),
                                MessageId(eval_msg_id),
                                PROCESSING_MESSAGE_TEXT,
                            )
                            .parse_mode(ParseMode::HTML),
                        )
                        .await?;
                    if edit_res.result.is_none() {
                        processing.eval_msg_id = None;
                        continue;
                    }
                    break eval_msg_id;
                }
                None => {
                    let send_msg_res = self
                        .client
                        .call_method_with_param::<_, Message>(
                            "sendMessage",
                            &SendMessage::new(chat_target.clone(), PROCESSING_MESSAGE_TEXT)
                                .parse_mode(ParseMode::HTML)
                                .reply(user_msg_id),
                        )
                        .await?
                        .into_result()?;
                    let eval_msg_id = send_msg_res.message_id.0;
                    processing.update_eval_msg_id(eval_msg_id).await;
                    break eval_msg_id;
                }
            }
        };
        let client = self.client.clone();
        let cancel_event = self.cancel_event.listen();
        spawn(async move {
            let res =
                continue_processing(chat_target, eval_msg_id, client, processing, cancel_event)
                    .await;
            if let Err(e) = res {
                error!("Error in continue_processing: {:?}", e);
            }
        })
        .detach();
        Ok(())
    }
}

async fn continue_processing(
    chat_target: ChatTarget<'static>,
    eval_msg_id: i64,
    client: Arc<TgClient>,
    wait_for_eval_result: impl WaitForEvalResult,
    cancel_event: EventListener,
) -> HandlerResult<()> {
    let eval_msg_id = MessageId(eval_msg_id);
    let res = wait_for_eval_result
        .wait_for_eval_result(cancel_event)
        .await;
    let data = match res {
        EvalResultResponse::Cancelled | EvalResultResponse::RequestOutdated => return Ok(()),
        EvalResultResponse::Err(e) => {
            client
                .call_method_with_param::<_, Message>(
                    "editMessageText",
                    &EditMessageText::new(
                        chat_target,
                        eval_msg_id,
                        format!("<i>Error: {}</i>", htmlize::escape_text(e)),
                    )
                    .parse_mode(ParseMode::HTML),
                )
                .await?;
            return Ok(());
        }
        EvalResultResponse::Ok(data) => data,
    };
    let (text, keyboard) = render_page_data(data);
    client
        .call_method_with_param::<_, Message>(
            "editMessageText",
            &EditMessageText::new(chat_target, eval_msg_id, text)
                .parse_mode(ParseMode::HTML)
                .reply_markup(keyboard),
        )
        .await?;
    Ok(())
}
