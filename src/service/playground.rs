use std::{future::Future, sync::Arc};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlaygroundError {
    #[error("request error")]
    Request(cyper::Error),
    #[error("timeout")]
    Timeout,
}

impl From<cyper::Error> for PlaygroundError {
    fn from(e: cyper::Error) -> Self {
        if let cyper::Error::Timeout = e {
            PlaygroundError::Timeout
        } else {
            PlaygroundError::Request(e)
        }
    }
}

pub type PlaygroundResult<T> = Result<T, PlaygroundError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaygroundExecuteResult {
    pub result_success: bool,
    pub result_code: String,
    pub result_exit_detail: String,
    pub result_stdout: String,
    pub result_stderr: String,
}

pub trait IPlaygrounService {
    fn run_code(
        &self,
        code: &str,
        channel: &'static str,
        mode: &'static str,
        edition: &'static str,
    ) -> impl Future<Output = PlaygroundResult<PlaygroundExecuteResult>>;
    // TODO: run miri
    fn generate_link(
        &self,
        code: &str,
        channel: &'static str,
        mode: &'static str,
        edition: &'static str,
    ) -> impl Future<Output = PlaygroundResult<String>>;
}

#[derive(Clone)]
pub struct PlaygroundService {
    base_url: Arc<String>,
    client: cyper::Client,
}

impl PlaygroundService {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url: Arc::new(base_url),
            client: cyper::Client::new(),
        }
    }
}

impl IPlaygrounService for PlaygroundService {
    async fn run_code(
        &self,
        code: &str,
        channel: &'static str,
        mode: &'static str,
        edition: &'static str,
    ) -> PlaygroundResult<PlaygroundExecuteResult> {
        #[derive(Clone, Debug, Serialize)]
        struct RunRequest<'a> {
            channel: &'a str,
            mode: &'a str,
            edition: &'a str,
            #[serde(rename = "crateType")]
            crate_type: &'a str,
            tests: bool,
            // #[serde(default)]
            // backtrace: bool,
            code: &'a str,
        }
        #[derive(Clone, Debug, Deserialize)]
        struct RunResponse {
            pub(crate) success: bool,
            #[serde(rename = "exitDetail")]
            pub(crate) exit_detail: String,
            pub(crate) stdout: String,
            pub(crate) stderr: String,
        }
        let res = self
            .client
            .post(format!("{}/execute", self.base_url))?
            .json(&RunRequest {
                channel,
                mode,
                edition,
                crate_type: "bin",
                tests: false,
                code,
            })?
            .send()
            .await?;
        let text = res.text().await?;
        let result: RunResponse = serde_json::from_str(&text).map_err(cyper::Error::Json)?;
        Ok(PlaygroundExecuteResult {
            result_success: result.success,
            result_code: "".into(),
            result_exit_detail: result.exit_detail,
            result_stdout: result.stdout,
            result_stderr: result.stderr,
        })
    }

    async fn generate_link(
        &self,
        code: &str,
        channel: &'static str,
        mode: &'static str,
        edition: &'static str,
    ) -> PlaygroundResult<String> {
        #[derive(Clone, Debug, Serialize)]
        struct CreateGistRequest<'a> {
            code: &'a str,
        }
        #[derive(Clone, Debug, Deserialize)]
        struct GistResponse<'a> {
            pub(crate) id: &'a str,
            // pub(crate) url: String,
            // pub(crate) code: String,
        }
        let res = self
            .client
            .post(format!("{}/meta/gist", self.base_url))?
            .json(&CreateGistRequest { code })?
            .send()
            .await?;
        let text = res.text().await?;
        let result: GistResponse = serde_json::from_str(&text).map_err(cyper::Error::Json)?;
        Ok(format!(
            "https://play.rust-lang.org/?version={channel}&mode={mode}&edition={edition}&gist={}",
            result.id
        ))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[compio::test]
    async fn test_run_code() {
        let service = PlaygroundService::new("https://play.rust-lang.org".into());
        let res = service
            .run_code(
                r#"fn main() { println!("Hello, world!"); }"#,
                "stable",
                "debug",
                "2021",
            )
            .await
            .unwrap();
        assert!(res.result_success);
        assert_eq!(res.result_stdout, "Hello, world!\n");
    }

    #[compio::test]
    async fn test_playground() {
        // "stderr":"   Compiling playground v0.0.1 (/playground)\n    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.67s\n     Running `target/debug/playground`\n"
        // "stderr":"   Compiling playground v0.0.1 (/playground)\nwarning: unused variable: `a`\n --> src/main.rs:1:17\n  |\n1 | fn main() { let a = 1; println!(\"Hello, world!\"); }\n  |                 ^ help: if this is intentional, prefix it with an underscore: `_a`\n  |\n  = note: `#[warn(unused_variables)]` on by default\n\nwarning: `playground` (bin \"playground\") generated 1 warning\n    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.76s\n     Running `target/debug/playground`\n"
        // "stderr":"   Compiling playground v0.0.1 (/playground)\nerror[E0277]: cannot add `&str` to `{integer}`\n --> src/main.rs:1:24\n  |\n1 | fn main() { let a = 1;a+\"\"; println!(\"Hello, world!\"); }\n  |
        //      ^ no implementation for `{integer} + &str`\n  |\n  = help: the trait `Add<&str>` is not implemented for `{integer}`\n  = help: the following other types implement trait `Add<Rhs>`:\n            `&f128` implements `Add<f128>`\n            `&f128` implements `Add`\n            `&f16` implements `Add<f16>`\n            `&f16` implements `Add`\n            `&f32` implements `Add<f32>`\n            `&f32` implements `Add`\n            `&f64` implements `Add<f64>`\n            `&f64` implements `Add`\n          and 56 others\n\nFor more information about this error, try `rustc --explain E0277`.\nerror: could not compile `playground` (bin \"playground\") due to 1 previous error\n"
        let client = cyper::Client::new();
        let res = client
            .post("https://play.rust-lang.org/execute")
            .unwrap()
            .json(&json!({
                "channel": "stable",
                "mode": "debug",
                "edition": "2021",
                "crateType": "bin",
                "tests": false,
                "code": r#"fn main() { let a = 1;a+""; println!("Hello, world!"); }"#
            }))
            .unwrap()
            .send()
            .await
            .unwrap();
        let text = res.text().await.unwrap();
        println!("{text}");
    }

    #[compio::test]
    async fn test_generate_link() {
        let service = PlaygroundService::new("https://play.rust-lang.org".into());
        let res = service
            .generate_link(
                r#"fn main() { println!("Hello, world!"); }"#,
                "stable",
                "debug",
                "2021",
            )
            .await
            .unwrap();
        assert!(res.starts_with(
            "https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist="
        ));
    }
}
