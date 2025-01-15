use tracing::error;

use super::*;
use wait_eval::EvalProcessingResponseImpl;

impl<R: IPlaygroundRecordRepository, P> super::UpdateEvalMsgId
    for EvalProcessingResponseImpl<R, P>
{
    async fn update_eval_msg_id(&self, eval_msg_id: i64) {
        let res = self
            .controller
            .repo
            .update_eval_msg_id_for_revision_id(self.upsert_result.revision_id, eval_msg_id)
            .await;
        if let Err(e) = res {
            error!("Failed to update eval_msg_id: {}", e);
        }
    }
}
