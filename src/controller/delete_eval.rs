use tracing::error;

use super::*;
use crate::repository::playground_record::PlaygroundRecordRevisionId;

impl<R: IPlaygroundRecordRepository, P> Controller<R, P>
where
    Self: Clone,
{
    pub(super) async fn delete_eval(
        &self,
        eval_msg_id: i64,
        request_user_id: i64,
        revision_id: i64,
    ) -> RequestDeleteEvalResponse<RevertDeleteEvalImpl<R, P>> {
        let Ok(revision_id) = PlaygroundRecordRevisionId::try_from(revision_id) else {
            return RequestDeleteEvalResponse::SenderMismatch;
        };
        let res = self
            .repo
            .delete_record_by_revision_id_if_match(eval_msg_id, request_user_id, revision_id)
            .await;
        match res {
            Ok(true) => RequestDeleteEvalResponse::Approved(RevertDeleteEvalImpl {
                eval_msg_id,
                revision_id,
                controller: self.clone(),
            }),
            Ok(false) => RequestDeleteEvalResponse::SenderMismatch,
            Err(e) => {
                error!("Failed to delete record: {}", e);
                RequestDeleteEvalResponse::Err("Error deleting eval record".into())
            }
        }
    }
}

pub struct RevertDeleteEvalImpl<R, P> {
    eval_msg_id: i64,
    revision_id: PlaygroundRecordRevisionId,
    controller: Controller<R, P>,
}

impl<R: IPlaygroundRecordRepository, P> RevertDeleteEval for RevertDeleteEvalImpl<R, P> {
    async fn revert_delete_eval(&self) {
        let res = self
            .controller
            .repo
            .update_eval_msg_id_for_revision_id(self.revision_id, self.eval_msg_id)
            .await;
        if let Err(e) = res {
            error!("Failed to revert delete: {}", e);
        }
    }
}
