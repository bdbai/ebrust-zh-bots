use super::EvalPageData;
use crate::repository::playground_record::{PlaygroundRecordPageState, PlaygroundRecordRevision};

pub(super) fn build_page_data(
    revision: PlaygroundRecordRevision,
    page_state: PlaygroundRecordPageState,
) -> EvalPageData {
    let mut data = EvalPageData {
        perma_link: revision.perma_link,
        has_warning: revision.warning_count > 0,
        has_error: !revision.result_success,
        has_fatal_error: !revision.playground_error.is_empty(),
        diagnostic_count: revision.error_count + revision.warning_count,
        revision: revision.record_revision_count,
        revision_id: revision.revision_id.0.get(),
        title: "",
        content: "".into(),
    };
    match (revision.playground_error.is_empty(), page_state) {
        (true, PlaygroundRecordPageState::Output) => {
            data.title = "Output";
            data.content = revision.result_stdout;
        }
        (true, PlaygroundRecordPageState::Stderr) => {
            data.title = "Stderr";
            data.content = revision.result_stderr;
        }
        _ => {
            data.title = "Error";
            data.content = revision.playground_error;
        }
    }
    data
}
