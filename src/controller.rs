use std::{
    fmt::{self, Debug},
    future::Future,
};

mod delete_eval;
mod eval_state;
mod get_eval_link;
mod new_eval;
mod page_data;
mod update_msg_id;
mod wait_eval;

use event_listener::EventListener;

use crate::{
    repository::playground_record::{
        CreateRevisionUpsertRecordResult, IPlaygroundRecordRepository,
    },
    service::playground::IPlaygrounService,
};

pub trait IController {
    type EvalProcessingImpl: WaitForEvalResult + UpdateEvalMsgId;
    type RevertDeleteEvalImpl: RevertDeleteEval;
    fn new_eval(
        &self,
        user_msg_id: i64,
        created_by_user_id: i64,
        code: String,
    ) -> impl Future<Output = EvalResponse<EvalProcessingResponse<Self::EvalProcessingImpl>>>;
    fn switch_eval_state(
        &self,
        eval_msg_id: i64,
        request_user_id: i64,
        revision_id: i64,
        request_page_state: EvalPageState,
    ) -> impl Future<Output = ShowEvalOutputResponse>;
    fn request_delete_eval(
        &self,
        eval_msg_id: i64,
        request_user_id: i64,
        revision_id: i64,
    ) -> impl Future<Output = RequestDeleteEvalResponse<Self::RevertDeleteEvalImpl>>;
    fn get_eval_link(&self, revision_id: i64) -> impl Future<Output = GetEvalLinkResponse>;
}

#[derive(Clone)]
pub struct EvalProcessingResponse<I> {
    imp: I,
    pub eval_msg_id: Option<i64>,
}

impl<I: Debug> Debug for EvalProcessingResponse<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EvalProcessingResponse")
            .field("imp", &self.imp)
            .field("eval_msg_id", &self.eval_msg_id)
            .finish()
    }
}

impl<I: WaitForEvalResult> WaitForEvalResult for EvalProcessingResponse<I> {
    fn wait_for_eval_result(
        self,
        cancel_event: EventListener,
    ) -> impl Future<Output = EvalResultResponse> {
        self.imp.wait_for_eval_result(cancel_event)
    }
}

impl<I: UpdateEvalMsgId> UpdateEvalMsgId for EvalProcessingResponse<I> {
    fn update_eval_msg_id(&self, eval_msg_id: i64) -> impl Future<Output = ()> {
        self.imp.update_eval_msg_id(eval_msg_id)
    }
}

#[derive(Debug, Clone)]
pub enum EvalResponse<R> {
    Processing(R),
    Err(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalPageData {
    pub perma_link: Option<String>,
    pub has_warning: bool,
    pub has_error: bool,
    pub has_fatal_error: bool,
    pub diagnostic_count: u32,
    pub revision: u32,
    pub revision_id: i64,
    pub title: &'static str,
    pub content: String,
}

pub trait WaitForEvalResult {
    fn wait_for_eval_result(
        self,
        cancel_event: EventListener,
    ) -> impl Future<Output = EvalResultResponse>;
}

pub trait UpdateEvalMsgId {
    fn update_eval_msg_id(&self, eval_msg_id: i64) -> impl Future<Output = ()>;
}

pub trait RevertDeleteEval {
    fn revert_delete_eval(&self) -> impl Future<Output = ()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalResultResponse {
    Ok(EvalPageData),
    RequestOutdated,
    Cancelled,
    Err(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShowEvalOutputResponse {
    Ok(EvalPageData),
    SenderMismatch,
    Err(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalPageState {
    Output,
    Build,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestDeleteEvalResponse<R> {
    Approved(R),
    SenderMismatch,
    Err(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GetEvalLinkResponse {
    Ok(EvalPageData),
    NotFound,
    Err(String),
}

#[derive(Clone)]
pub struct Controller<R, P> {
    repo: R,
    playground: P,
}

impl<R, P> Controller<R, P> {
    pub fn new(repo: R, service: P) -> Self {
        Self {
            repo,
            playground: service,
        }
    }
}

impl<R: IPlaygroundRecordRepository, P: IPlaygrounService> IController for Controller<R, P>
where
    Self: Clone,
{
    type EvalProcessingImpl = wait_eval::EvalProcessingResponseImpl<R, P>;
    type RevertDeleteEvalImpl = delete_eval::RevertDeleteEvalImpl<R, P>;

    fn new_eval(
        &self,
        user_msg_id: i64,
        created_by_user_id: i64,
        code: String,
    ) -> impl Future<Output = EvalResponse<EvalProcessingResponse<Self::EvalProcessingImpl>>> {
        self.new_eval(user_msg_id, created_by_user_id, code)
    }

    fn request_delete_eval(
        &self,
        eval_msg_id: i64,
        request_user_id: i64,
        revision_id: i64,
    ) -> impl Future<Output = RequestDeleteEvalResponse<Self::RevertDeleteEvalImpl>> {
        self.delete_eval(eval_msg_id, request_user_id, revision_id)
    }

    fn switch_eval_state(
        &self,
        eval_msg_id: i64,
        request_user_id: i64,
        revision_id: i64,
        request_page_state: EvalPageState,
    ) -> impl Future<Output = ShowEvalOutputResponse> {
        self.eval_state(
            eval_msg_id,
            request_user_id,
            revision_id,
            request_page_state,
        )
    }

    async fn get_eval_link(&self, revision_id: i64) -> GetEvalLinkResponse {
        self.get_eval_link(revision_id).await
    }
}
