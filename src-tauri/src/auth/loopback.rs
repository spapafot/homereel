use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use url::Url;

use crate::error::{AppError, AppResult};

const MAX_REQUEST_BYTES: usize = 16 * 1024;
const READ_TIMEOUT: Duration = Duration::from_secs(10);

const SUCCESS_PAGE: &str = "<!doctype html><html><head><meta charset=\"utf-8\"><title>DriveTube</title></head>\
<body style=\"background:#0f0f0f;color:#f1f1f1;font-family:Roboto,Arial,sans-serif;text-align:center;padding-top:20vh\">\
<h1>DriveTube is connected</h1><p>You can close this tab and return to the app.</p></body></html>";

const FAILURE_PAGE: &str = "<!doctype html><html><head><meta charset=\"utf-8\"><title>DriveTube</title></head>\
<body style=\"background:#0f0f0f;color:#f1f1f1;font-family:Roboto,Arial,sans-serif;text-align:center;padding-top:20vh\">\
<h1>DriveTube was not connected</h1><p>You can close this tab and try again from the app.</p></body></html>";

#[derive(Debug, PartialEq, Eq)]
pub enum CallbackOutcome {
    Code(String),
    Denied(String),
    /// A request that is not the OAuth redirect, such as `/favicon.ico`.
    Ignore,
    /// A redirect with a missing or mismatched `state`; never trusted.
    Invalid,
}

/// One-shot HTTP listener on 127.0.0.1 that receives Google's loopback redirect.
pub struct LoopbackListener {
    listener: TcpListener,
    port: u16,
}

impl LoopbackListener {
    pub async fn bind() -> AppResult<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        let port = listener.local_addr()?.port();
        Ok(Self { listener, port })
    }

    pub fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Waits until the browser delivers a redirect carrying `expected_state`.
    pub async fn wait_for_code(&self, expected_state: &str) -> AppResult<String> {
        loop {
            let (mut stream, _) = self.listener.accept().await?;
            let Ok(target) = read_request_target(&mut stream).await else {
                continue;
            };
            match parse_callback(&target, expected_state) {
                CallbackOutcome::Code(code) => {
                    respond(&mut stream, "200 OK", SUCCESS_PAGE).await;
                    return Ok(code);
                }
                CallbackOutcome::Denied(reason) => {
                    respond(&mut stream, "200 OK", FAILURE_PAGE).await;
                    return Err(if reason == "access_denied" {
                        AppError::Cancelled
                    } else {
                        AppError::internal(format!("Google returned an error: {reason}"))
                    });
                }
                CallbackOutcome::Ignore => respond(&mut stream, "404 Not Found", "").await,
                CallbackOutcome::Invalid => {
                    respond(&mut stream, "400 Bad Request", FAILURE_PAGE).await
                }
            }
        }
    }
}

pub fn parse_callback(request_target: &str, expected_state: &str) -> CallbackOutcome {
    let Ok(url) = Url::parse(&format!("http://127.0.0.1{request_target}")) else {
        return CallbackOutcome::Invalid;
    };
    if url.path() != "/" {
        return CallbackOutcome::Ignore;
    }
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    if params.get("state").map(String::as_str) != Some(expected_state) {
        return CallbackOutcome::Invalid;
    }
    if let Some(error) = params.get("error") {
        return CallbackOutcome::Denied(error.clone());
    }
    match params.get("code") {
        Some(code) if !code.is_empty() => CallbackOutcome::Code(code.clone()),
        _ => CallbackOutcome::Invalid,
    }
}

async fn read_request_target(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut request = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        if request.len() > MAX_REQUEST_BYTES {
            return Err(std::io::Error::other("request too large"));
        }
        let read = tokio::time::timeout(READ_TIMEOUT, stream.read(&mut chunk))
            .await
            .map_err(|_| std::io::Error::other("request timed out"))??;
        if read == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..read]);
    }
    let text = String::from_utf8_lossy(&request);
    let mut request_line = text.lines().next().unwrap_or_default().split_whitespace();
    match (request_line.next(), request_line.next()) {
        (Some("GET"), Some(target)) => Ok(target.to_owned()),
        _ => Err(std::io::Error::other("not a GET request")),
    }
}

async fn respond(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    // The browser tab may already be gone; the authorization result does not depend on it.
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_code_with_matching_state() {
        let outcome = parse_callback("/?state=abc&code=4%2F0Ab&scope=x", "abc");

        assert_eq!(outcome, CallbackOutcome::Code("4/0Ab".into()));
    }

    #[test]
    fn rejects_mismatched_or_missing_state() {
        assert_eq!(
            parse_callback("/?state=evil&code=1", "abc"),
            CallbackOutcome::Invalid
        );
        assert_eq!(parse_callback("/?code=1", "abc"), CallbackOutcome::Invalid);
    }

    #[test]
    fn reports_user_denial() {
        let outcome = parse_callback("/?state=abc&error=access_denied", "abc");

        assert_eq!(outcome, CallbackOutcome::Denied("access_denied".into()));
    }

    #[test]
    fn ignores_unrelated_paths() {
        assert_eq!(
            parse_callback("/favicon.ico", "abc"),
            CallbackOutcome::Ignore
        );
    }

    #[tokio::test]
    async fn ignores_stray_requests_then_returns_the_code() {
        let listener = LoopbackListener::bind().await.unwrap();
        let base = listener.redirect_uri();
        let browser = tokio::spawn(async move {
            let http = reqwest::Client::new();
            let favicon = http
                .get(format!("{base}/favicon.ico"))
                .send()
                .await
                .unwrap();
            let redirect = http
                .get(format!("{base}/?state=s1&code=c1"))
                .send()
                .await
                .unwrap();
            (favicon.status(), redirect.status())
        });

        let code = listener.wait_for_code("s1").await.unwrap();

        let (favicon_status, redirect_status) = browser.await.unwrap();
        assert_eq!(code, "c1");
        assert_eq!(favicon_status, reqwest::StatusCode::NOT_FOUND);
        assert_eq!(redirect_status, reqwest::StatusCode::OK);
    }
}
