use telegram_types::bot::methods::ApiError;
use thiserror::Error;

#[derive(Debug, Error)]
pub(super) enum HandlerError {
    #[error("request error: {0}")]
    Request(#[from] cyper::Error),
    #[error("api error: {0}")]
    Telegram(#[from] ApiError),
}

pub(super) type HandlerResult<T> = Result<T, HandlerError>;
