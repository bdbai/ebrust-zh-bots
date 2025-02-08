use tracing::error;

use super::*;
use crate::repository::playground_record::PlaygroundRecordPageState;
use wait_eval::EvalProcessingResponseImpl;

impl<R: IPlaygroundRecordRepository, P: IPlaygrounService> Controller<R, P>
where
    Self: Clone,
{
    pub(super) async fn new_eval(
        &self,
        chat_id: i64,
        user_msg_id: i64,
        created_by_user_id: i64,
        code: String,
    ) -> EvalResponse<EvalProcessingResponse<EvalProcessingResponseImpl<R, P>>> {
        let full_code = format!(
            "fn main() {{ let res = {{
            {} 
            }}; println!(\"{{res:?}}\"); }}",
            &code
        );
        let res = match self
            .repo
            .create_revision_upsert_record(
                chat_id,
                user_msg_id,
                created_by_user_id,
                full_code.clone(),
                PlaygroundRecordPageState::Output,
            )
            .await
        {
            Ok(res) => res,
            Err(e) => {
                error!("Failed to create record: {}", e);
                return EvalResponse::Err("Failed to create record".into());
            }
        };
        EvalResponse::Processing(EvalProcessingResponse {
            eval_msg_id: res.eval_msg_id,
            imp: EvalProcessingResponseImpl {
                rendered_code: full_code,
                controller: self.clone(),
                upsert_result: res,
            },
        })
    }
}
