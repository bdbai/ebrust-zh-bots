use std::fmt::Debug;

use cyper::Client;
use cyper::Error as CyperError;
use serde::{de::DeserializeOwned, Serialize};
use telegram_types::bot::methods::TelegramResult;
use tracing::instrument;

#[derive(Debug, Clone)]
pub(crate) struct TgClient {
    client: Client,
    server_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum TgEnv {
    Test,
    Prod,
}

impl TgClient {
    pub fn new(tg_api_key: String, env: TgEnv) -> Self {
        let server_url = match env {
            TgEnv::Test => format!("https://api.telegram.org/bot{tg_api_key}/test/"),
            TgEnv::Prod => format!("https://api.telegram.org/bot{tg_api_key}/"),
        };
        Self {
            client: Client::new(),
            server_url,
        }
    }

    #[instrument]
    pub async fn call_method<R: DeserializeOwned + Debug>(
        &self,
        method: &str,
    ) -> Result<TelegramResult<R>, CyperError> {
        let req = self.client.get(format!("{}{method}", &self.server_url))?;
        let res = req.send().await?;
        let res = res.json().await?;
        Ok(res)
    }

    #[instrument]
    pub async fn call_method_with_param<Q: Serialize + Debug, R: DeserializeOwned + Debug>(
        &self,
        method: &str,
        param: &Q,
    ) -> Result<TelegramResult<R>, CyperError> {
        let req = self.client.post(format!("{}{method}", &self.server_url))?;
        let res = req.json(&param)?.send().await?;
        let res = res.json().await?;
        Ok(res)
    }
}
