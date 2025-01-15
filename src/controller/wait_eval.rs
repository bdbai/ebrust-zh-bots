use std::pin::pin;

use futures::future::{select, Either};
use tracing::error;

use super::*;
use crate::repository::playground_record::{PlaygroundRecordPageState, PlaygroundRecordRevision};
use page_data::build_page_data;

#[derive(Clone)]
pub struct EvalProcessingResponseImpl<R, P> {
    pub(super) rendered_code: String,
    pub(super) controller: Controller<R, P>,
    pub(super) upsert_result: CreateRevisionUpsertRecordResult,
}

impl<R, P> Debug for EvalProcessingResponseImpl<R, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EvalProcessingResponse")
            .field("code", &self.rendered_code)
            .field("upsert_result", &self.upsert_result)
            .finish()
    }
}

impl<R: IPlaygroundRecordRepository, P: IPlaygrounService> WaitForEvalResult
    for EvalProcessingResponseImpl<R, P>
{
    async fn wait_for_eval_result(self, cancel_event: EventListener) -> EvalResultResponse {
        let Either::Left((res, _)) = select(
            pin!(async {
                self.controller
                    .playground
                    .run_code(&self.rendered_code, "stable", "debug", "2021")
                    .await
            }),
            cancel_event,
        )
        .await
        else {
            return EvalResultResponse::Cancelled;
        };

        let mut revision = match res {
            Err(e) => PlaygroundRecordRevision {
                revision_id: self.upsert_result.revision_id,
                rendered_code: self.rendered_code,
                playground_error: e.to_string(),
                ..Default::default()
            },
            Ok(res) => {
                let (mut error_count, mut warning_count) = (0, 0);
                for line in res.result_stderr.lines() {
                    if line.starts_with("error:") {
                        error_count += 1;
                    } else if line.starts_with("warning:") {
                        warning_count += 1;
                    }
                }
                if error_count > 1 {
                    error_count -= 1;
                }
                PlaygroundRecordRevision {
                    revision_id: self.upsert_result.revision_id,
                    rendered_code: self.rendered_code,
                    warning_count,
                    error_count,
                    result_success: res.result_success,
                    result_code: res.result_code,
                    result_exit_detail: res.result_exit_detail,
                    result_stdout: res.result_stdout,
                    result_stderr: res.result_stderr,
                    playground_error: "".to_string(),
                    ..Default::default()
                }
            }
        };
        let init_page_state = if revision.result_stdout.is_empty() {
            PlaygroundRecordPageState::Stderr
        } else {
            self.upsert_result.page_state
        };
        match self
            .controller
            .repo
            .update_revision_for_revision_count_and_is_latest(&mut revision)
            .await
        {
            Err(e) => {
                error!(
                    "Error in update_revision_for_revision_count_and_is_latest: {:?}",
                    e
                );
                EvalResultResponse::Err("Failed to update revision".into())
            }
            Ok(false) => EvalResultResponse::RequestOutdated,
            Ok(true) => EvalResultResponse::Ok(build_page_data(revision, init_page_state)),
        }
    }
}
