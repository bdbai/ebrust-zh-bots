use telegram_types::bot::{
    methods::{AnswerCallbackQuery, ChatTarget, DeleteMessage, EditMessageText, TelegramResult},
    types::{CallbackQuery, Message, ParseMode},
};
use tracing::error;

use crate::controller::{
    EvalPageState, GetEvalLinkResponse, IController, RequestDeleteEvalResponse, RevertDeleteEval,
    ShowEvalOutputResponse,
};

use super::{error::HandlerResult, render::render_page_data, Handler};

impl<'e, C: IController> Handler<'e, C> {
    pub(super) async fn handle_message_callback_query(
        &mut self,
        query: CallbackQuery,
    ) -> HandlerResult<()> {
        let query_id = query.id.clone();
        let res = self.handle_message_callback_query_inner(query).await;
        let text = match res {
            Ok(true) => return Ok(()),
            Ok(false) => "Unknown command",
            Err(e) => {
                error!(error = %e, "Error handling callback query");
                "Internal error"
            }
        };
        self.client
            .call_method_with_param::<_, bool>(
                "answerCallbackQuery",
                &AnswerCallbackQuery::new(query_id).text(text.into()),
            )
            .await?;
        Ok(())
    }
    async fn handle_message_callback_query_inner(
        &mut self,
        mut query: CallbackQuery,
    ) -> HandlerResult<bool> {
        let payload = query.data.take().unwrap_or_default();
        let Some(("v1", rem)) = payload.split_once(':') else {
            return Ok(false);
        };
        let Some((action, rem)) = rem.split_once(':') else {
            return Ok(false);
        };
        match action {
            "del" => self.handle_delete_eval(rem, query).await,
            "state" => self.handle_eval_state(rem, query).await,
            "genlink" => self.handle_gen_link(rem, query).await,
            _ => Ok(false),
        }
    }
    async fn handle_delete_eval(&mut self, rem: &str, query: CallbackQuery) -> HandlerResult<bool> {
        let Some(msg) = query.message.as_ref() else {
            return Ok(false);
        };
        let Some(revision_id) = rem.parse::<i64>().ok() else {
            return Ok(false);
        };
        let from_id = query.from.id.0;
        let res = self
            .controller
            .request_delete_eval(msg.message_id.0, from_id, revision_id)
            .await;
        let mut query_response = AnswerCallbackQuery::new(query.id);
        query_response = match res {
            RequestDeleteEvalResponse::Approved(rev) => {
                let res = self
                    .client
                    .call_method_with_param::<_, bool>(
                        "deleteMessage",
                        &DeleteMessage {
                            chat_id: ChatTarget::id(msg.chat.id.0),
                            message_id: msg.message_id,
                        },
                    )
                    .await;
                if !matches!(res, Ok(TelegramResult { ok: true, .. })) {
                    error!(
                        msg_id = %msg.message_id.0,
                        from_id = %from_id,
                        chat_id = %msg.chat.id.0,
                        res = ?res,
                        "Failed to delete message"
                    );
                    rev.revert_delete_eval().await;
                }
                query_response
            }
            RequestDeleteEvalResponse::SenderMismatch => {
                query_response.text("Only the original sender can delete".into())
            }
            RequestDeleteEvalResponse::Err(e) => query_response.text(e).show_alert(true),
        };
        self.client
            .call_method_with_param::<_, bool>("answerCallbackQuery", &query_response)
            .await?;
        Ok(true)
    }

    async fn handle_eval_state(&mut self, rem: &str, query: CallbackQuery) -> HandlerResult<bool> {
        let Some(msg) = query.message.as_ref() else {
            return Ok(false);
        };
        let (request_page_state, rem) = match rem.split_once(':') {
            Some(("output", rem)) => (EvalPageState::Output, rem),
            Some(("build", rem)) => (EvalPageState::Build, rem),
            _ => return Ok(false),
        };
        let Some(revision_id) = rem.parse::<i64>().ok() else {
            return Ok(false);
        };
        let from_id = query.from.id.0;
        let res = self
            .controller
            .switch_eval_state(msg.message_id.0, from_id, revision_id, request_page_state)
            .await;
        let mut query_response = AnswerCallbackQuery::new(query.id);
        query_response = match res {
            ShowEvalOutputResponse::Ok(page_data) => {
                let (text, keyboard) = render_page_data(page_data);
                self.client
                    .call_method_with_param::<_, Message>(
                        "editMessageText",
                        &EditMessageText::new(ChatTarget::id(msg.chat.id.0), msg.message_id, text)
                            .parse_mode(ParseMode::HTML)
                            .reply_markup(keyboard),
                    )
                    .await?;
                query_response
            }
            ShowEvalOutputResponse::SenderMismatch => {
                query_response.text("Only the original sender can switch state".into())
            }
            ShowEvalOutputResponse::Err(e) => query_response.text(e).show_alert(true),
        };
        self.client
            .call_method_with_param::<_, bool>("answerCallbackQuery", &query_response)
            .await?;
        Ok(true)
    }

    async fn handle_gen_link(&mut self, rem: &str, query: CallbackQuery) -> HandlerResult<bool> {
        let Some(msg) = query.message.as_ref() else {
            return Ok(false);
        };
        let Some(revision_id) = rem.parse::<i64>().ok() else {
            return Ok(false);
        };
        let res = self.controller.get_eval_link(revision_id).await;
        let mut query_response = AnswerCallbackQuery::new(query.id);
        query_response = match res {
            GetEvalLinkResponse::Ok(page_data) => {
                let (text, keyboard) = render_page_data(page_data);
                self.client
                    .call_method_with_param::<_, Message>(
                        "editMessageText",
                        &EditMessageText::new(ChatTarget::id(msg.chat.id.0), msg.message_id, text)
                            .parse_mode(ParseMode::HTML)
                            .reply_markup(keyboard),
                    )
                    .await?;
                query_response.text("Link generated in-place".into())
            }
            GetEvalLinkResponse::NotFound => query_response
                .text("Revision not found".into())
                .show_alert(true),
            GetEvalLinkResponse::Err(e) => query_response.text(e).show_alert(true),
        };
        self.client
            .call_method_with_param::<_, bool>("answerCallbackQuery", &query_response)
            .await?;
        Ok(true)
    }
}
