use tracing::error;

use super::*;
use crate::repository::playground_record::PlaygroundRecordRevisionId;
use page_data::build_page_data;

impl<R: IPlaygroundRecordRepository, P: IPlaygrounService> Controller<R, P> {
    pub(super) async fn get_eval_link(&self, revision_id: i64) -> GetEvalLinkResponse {
        let Ok(revision_id) = PlaygroundRecordRevisionId::try_from(revision_id) else {
            return GetEvalLinkResponse::NotFound;
        };
        let (mut revision, page_state) = match self.repo.get_revision_by_id(revision_id).await {
            Ok(Some(revision)) => revision,
            Ok(None) => return GetEvalLinkResponse::NotFound,
            Err(e) => {
                error!("Failed to get revision: {}", e);
                return GetEvalLinkResponse::Err("Error getting revision".into());
            }
        };

        if revision.perma_link.is_some() {
            return GetEvalLinkResponse::Ok(build_page_data(revision, page_state));
        }

        let res = self
            .playground
            .generate_link(&revision.rendered_code, "stable", "debug", "2021")
            .await;
        match res {
            Ok(link) => {
                let res = self
                    .repo
                    .update_perma_link_for_revision_id(revision.revision_id, link.clone())
                    .await;
                if let Err(e) = res {
                    error!("Failed to update perma_link: {}", e);
                }
                revision.perma_link = Some(link);
                GetEvalLinkResponse::Ok(build_page_data(revision, page_state))
            }
            Err(e) => {
                error!("Failed to generate link: {}", e);
                GetEvalLinkResponse::Err("Error generating link".into())
            }
        }
    }
}
