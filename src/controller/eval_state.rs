use tracing::error;

use super::*;
use crate::repository::playground_record::{PlaygroundRecordPageState, PlaygroundRecordRevisionId};
use page_data::build_page_data;

impl<R: IPlaygroundRecordRepository, P> Controller<R, P> {
    pub(super) async fn eval_state(
        &self,
        eval_msg_id: i64,
        request_user_id: i64,
        revision_id: i64,
        request_page_state: EvalPageState,
    ) -> ShowEvalOutputResponse {
        let Ok(revision_id) = PlaygroundRecordRevisionId::try_from(revision_id) else {
            return ShowEvalOutputResponse::SenderMismatch;
        };
        let request_page_state = match request_page_state {
            EvalPageState::Output => PlaygroundRecordPageState::Output,
            EvalPageState::Build => PlaygroundRecordPageState::Stderr,
        };
        let res = self
            .repo
            .get_revision_update_page_state_if_match(
                eval_msg_id,
                request_user_id,
                revision_id,
                request_page_state,
            )
            .await;
        match res {
            Ok(Some(revision)) => {
                ShowEvalOutputResponse::Ok(build_page_data(revision, request_page_state))
            }
            Ok(None) => ShowEvalOutputResponse::SenderMismatch,
            Err(e) => {
                error!("Failed to switch eval state: {}", e);
                ShowEvalOutputResponse::Err("Error lookup eval record".into())
            }
        }
    }
}
