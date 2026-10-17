use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use reqwest::StatusCode;
use serde::Deserialize;

pub const DEFAULT_MODEL: &str = "gemini-3.8-flash";
pub const API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
/// Room for low-effort thinking plus the small JSON answer.
const MAX_OUTPUT_TOKENS: u32 = 1024;
/// Model families that cannot write a text title from images.
const NON_TITLE_MODELS: [&str; 6] = ["tts", "image", "audio", "live", "omni", "embedding"];

#[derive(Debug, thiserror::Error)]
pub enum GeminiError {
    #[error("The Gemini API key is missing or invalid. Check it in Settings.")]
    InvalidKey,
    #[error("The Gemini model \"{0}\" is not available. Choose another model in Settings.")]
    ModelNotFound(String),
    #[error("Gemini is limiting requests right now.")]
    RateLimited { retry_after: Option<Duration> },
    #[error(
        "Your Gemini prepaid credits have run out. Add credits in Google AI Studio, then generate titles again."
    )]
    NoCredits,
    #[error("Gemini is temporarily unavailable ({0}).")]
    Unavailable(String),
    #[error("Gemini rejected the request: {0}")]
    Rejected(String),
    #[error("Unexpected Gemini response: {0}")]
    BadResponse(String),
}

impl GeminiError {
    /// Errors worth retrying after a pause; the rest fail immediately.
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            GeminiError::RateLimited { .. } | GeminiError::Unavailable(_)
        )
    }

    /// Errors that affect every request, so the whole queue should stop.
    pub fn stops_queue(&self) -> bool {
        matches!(
            self,
            GeminiError::InvalidKey | GeminiError::ModelNotFound(_) | GeminiError::NoCredits
        )
    }
}

pub struct Image {
    pub mime_type: &'static str,
    pub bytes: Vec<u8>,
}

/// One piece of a request, sent in order, so text can label the images that follow it.
pub enum PromptPart {
    Text(String),
    Image(Image),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TitleSuggestion {
    pub title: String,
    pub confidence: f64,
    /// Names from the request's reference people that the model recognized.
    pub people: Vec<String>,
    pub prompt_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

/// Stateless `generateContent` client. The API key travels in a header, never in the URL, and is
/// only ever held by the Rust layer.
pub struct GeminiClient {
    http: reqwest::Client,
    api_base: String,
}

#[derive(Deserialize)]
struct ModelList {
    #[serde(default)]
    models: Vec<ModelInfo>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelInfo {
    name: String,
    #[serde(default)]
    supported_generation_methods: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenerateResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    content: Option<Content>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct Content {
    #[serde(default)]
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize)]
struct ResponsePart {
    text: Option<String>,
    #[serde(default)]
    thought: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageMetadata {
    prompt_token_count: Option<i64>,
    candidates_token_count: Option<i64>,
    thoughts_token_count: Option<i64>,
}

#[derive(Deserialize)]
struct TitleJson {
    title: String,
    #[serde(default)]
    confidence: f64,
    #[serde(default)]
    people: Vec<String>,
}

#[derive(Deserialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Deserialize)]
struct ErrorDetail {
    #[serde(default)]
    message: String,
    #[serde(default)]
    details: Vec<serde_json::Value>,
}

impl GeminiClient {
    pub fn new(http: reqwest::Client, api_base: &str) -> Self {
        Self {
            http,
            api_base: api_base.trim_end_matches('/').to_owned(),
        }
    }

    /// Flash-tier models this key can use for titles, newest first. Also validates the key.
    pub async fn list_title_models(&self, api_key: &str) -> Result<Vec<String>, GeminiError> {
        let response = self
            .http
            .get(format!("{}/models", self.api_base))
            .query(&[("pageSize", "200")])
            .header("x-goog-api-key", api_key)
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(network_error)?;
        let response = check_status(response, "").await?;
        let list: ModelList = response
            .json()
            .await
            .map_err(|error| GeminiError::BadResponse(error.without_url().to_string()))?;
        let mut models: Vec<String> = list
            .models
            .into_iter()
            .filter(|model| is_title_model(&model.name, &model.supported_generation_methods))
            .map(|model| model.name.trim_start_matches("models/").to_owned())
            .collect();
        models.sort_by(|a, b| b.cmp(a));
        Ok(models)
    }

    /// `people_names` are the reference people in `parts`; the answer can only name them.
    pub async fn suggest_title(
        &self,
        api_key: &str,
        model: &str,
        parts: &[PromptPart],
        people_names: &[String],
    ) -> Result<TitleSuggestion, GeminiError> {
        let parts: Vec<serde_json::Value> = parts
            .iter()
            .map(|part| match part {
                PromptPart::Text(text) => serde_json::json!({ "text": text }),
                PromptPart::Image(image) => serde_json::json!({
                    "inline_data": { "mime_type": image.mime_type, "data": STANDARD.encode(&image.bytes) }
                }),
            })
            .collect();
        let body = serde_json::json!({
            "contents": [{ "role": "user", "parts": parts }],
            "generationConfig": {
                "responseMimeType": "application/json",
                "responseJsonSchema": response_schema(people_names),
                // Low media resolution keeps five frames near 1,300 input tokens.
                "mediaResolution": "MEDIA_RESOLUTION_LOW",
                "thinkingConfig": { "thinkingLevel": "low" },
                "maxOutputTokens": MAX_OUTPUT_TOKENS
            }
        });

        let response = self
            .http
            .post(format!("{}/models/{model}:generateContent", self.api_base))
            .header("x-goog-api-key", api_key)
            .timeout(REQUEST_TIMEOUT)
            .json(&body)
            .send()
            .await
            .map_err(network_error)?;
        let response = check_status(response, model).await?;
        let parsed: GenerateResponse = response
            .json()
            .await
            .map_err(|error| GeminiError::BadResponse(error.without_url().to_string()))?;
        parse_suggestion(parsed)
    }
}

fn response_schema(people_names: &[String]) -> serde_json::Value {
    if people_names.is_empty() {
        return serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "confidence": { "type": "number", "minimum": 0, "maximum": 1 }
            },
            "required": ["title", "confidence"]
        });
    }
    // The enum keeps answers to the names the user gave. JSON Schema does not allow an empty one,
    // hence the separate schema above.
    serde_json::json!({
        "type": "object",
        "properties": {
            "title": { "type": "string" },
            "confidence": { "type": "number", "minimum": 0, "maximum": 1 },
            "people": { "type": "array", "items": { "type": "string", "enum": people_names } }
        },
        "required": ["title", "confidence", "people"]
    })
}

fn parse_suggestion(response: GenerateResponse) -> Result<TitleSuggestion, GeminiError> {
    let candidate = response
        .candidates
        .into_iter()
        .next()
        .ok_or_else(|| GeminiError::BadResponse("no answer was returned".into()))?;
    let text: String = candidate
        .content
        .map(|content| {
            content
                .parts
                .into_iter()
                .filter(|part| !part.thought)
                .filter_map(|part| part.text)
                .collect()
        })
        .unwrap_or_default();
    if text.trim().is_empty() {
        let reason = candidate.finish_reason.unwrap_or_else(|| "unknown".into());
        return Err(GeminiError::BadResponse(format!(
            "empty answer (finish reason {reason})"
        )));
    }
    let json: TitleJson = serde_json::from_str(&text).map_err(|error| {
        GeminiError::BadResponse(format!("answer was not the expected JSON: {error}"))
    })?;
    let usage = response.usage_metadata;
    Ok(TitleSuggestion {
        title: json.title,
        confidence: json.confidence.clamp(0.0, 1.0),
        people: json.people,
        prompt_tokens: usage.as_ref().and_then(|u| u.prompt_token_count),
        // Thinking tokens are billed as output.
        output_tokens: usage
            .as_ref()
            .map(|u| u.candidates_token_count.unwrap_or(0) + u.thoughts_token_count.unwrap_or(0)),
    })
}

pub fn is_title_model(name: &str, methods: &[String]) -> bool {
    name.contains("flash")
        && methods.iter().any(|method| method == "generateContent")
        && !NON_TITLE_MODELS.iter().any(|family| name.contains(family))
}

fn network_error(error: reqwest::Error) -> GeminiError {
    GeminiError::Unavailable(error.without_url().to_string())
}

async fn check_status(
    response: reqwest::Response,
    model: &str,
) -> Result<reqwest::Response, GeminiError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let detail = response
        .json::<ErrorBody>()
        .await
        .ok()
        .map(|body| body.error);
    Err(classify_error(status, detail, model))
}

fn classify_error(status: StatusCode, detail: Option<ErrorDetail>, model: &str) -> GeminiError {
    let details = detail
        .as_ref()
        .map(|d| d.details.as_slice())
        .unwrap_or_default();
    let reason = details
        .iter()
        .find_map(|item| item.get("reason").and_then(|r| r.as_str()));
    let message = detail
        .as_ref()
        .map(|d| d.message.clone())
        .unwrap_or_default();

    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => GeminiError::InvalidKey,
        StatusCode::BAD_REQUEST if reason == Some("API_KEY_INVALID") => GeminiError::InvalidKey,
        StatusCode::NOT_FOUND => GeminiError::ModelNotFound(model.to_owned()),
        // Google moved this from 429 to 402; some accounts still get 429 with the same message.
        StatusCode::PAYMENT_REQUIRED => GeminiError::NoCredits,
        StatusCode::TOO_MANY_REQUESTS if message.contains("credits are depleted") => {
            GeminiError::NoCredits
        }
        StatusCode::TOO_MANY_REQUESTS => GeminiError::RateLimited {
            retry_after: details
                .iter()
                .find_map(|item| item.get("retryDelay").and_then(|d| d.as_str()))
                .and_then(parse_retry_delay),
        },
        status if status.is_server_error() => GeminiError::Unavailable(status.to_string()),
        _ => GeminiError::Rejected(if message.is_empty() {
            status.to_string()
        } else {
            message
        }),
    }
}

/// Parses Google's `RetryInfo.retryDelay`, such as `"13s"` or `"1.5s"`.
fn parse_retry_delay(text: &str) -> Option<Duration> {
    let seconds: f64 = text.strip_suffix('s')?.parse().ok()?;
    (seconds.is_finite() && seconds >= 0.0).then(|| Duration::from_secs_f64(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_partial_json, body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn client(server: &MockServer) -> GeminiClient {
        GeminiClient::new(reqwest::Client::new(), &server.uri())
    }

    fn frame_parts() -> Vec<PromptPart> {
        vec![
            PromptPart::Text("Name this video".into()),
            PromptPart::Image(Image {
                mime_type: "image/jpeg",
                bytes: vec![0xFF, 0xD8, 0xFF],
            }),
        ]
    }

    #[tokio::test]
    async fn sends_frames_with_structured_output_and_parses_the_title() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/models/gemini-3.8-flash:generateContent"))
            .and(header("x-goog-api-key", "key-1"))
            .and(body_string_contains("\"responseMimeType\":\"application/json\""))
            .and(body_string_contains("MEDIA_RESOLUTION_LOW"))
            .and(body_string_contains("\"mime_type\":\"image/jpeg\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "candidates": [{
                    "content": { "parts": [
                        { "text": "Thinking about the frames", "thought": true },
                        { "text": "{\"title\": \"Kids building a sandcastle\", \"confidence\": 0.86}" }
                    ]},
                    "finishReason": "STOP"
                }],
                "usageMetadata": {
                    "promptTokenCount": 1438, "candidatesTokenCount": 26, "thoughtsTokenCount": 135
                }
            })))
            .expect(1)
            .mount(&server)
            .await;

        let suggestion = client(&server)
            .suggest_title("key-1", "gemini-3.8-flash", &frame_parts(), &[])
            .await
            .unwrap();

        assert_eq!(
            suggestion,
            TitleSuggestion {
                title: "Kids building a sandcastle".into(),
                confidence: 0.86,
                people: Vec::new(),
                prompt_tokens: Some(1438),
                output_tokens: Some(161),
            }
        );
    }

    #[tokio::test]
    async fn asks_which_reference_people_appear_and_parses_them() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_partial_json(serde_json::json!({
                "generationConfig": { "responseJsonSchema": {
                    "properties": { "people": {
                        "type": "array",
                        "items": { "type": "string", "enum": ["Maria", "Rex"] }
                    }},
                    "required": ["title", "confidence", "people"]
                }}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "candidates": [{
                    "content": { "parts": [
                        { "text": "{\"title\": \"Rex Chasing Waves\", \"confidence\": 0.9, \"people\": [\"Rex\"]}" }
                    ]}
                }]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let suggestion = client(&server)
            .suggest_title(
                "key-1",
                "gemini-3.8-flash",
                &frame_parts(),
                &["Maria".to_string(), "Rex".to_string()],
            )
            .await
            .unwrap();

        assert_eq!(suggestion.title, "Rex Chasing Waves");
        assert_eq!(suggestion.people, vec!["Rex"]);
    }

    #[tokio::test]
    async fn rate_limits_carry_the_suggested_retry_delay() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
                "error": {
                    "code": 429, "message": "Resource exhausted", "status": "RESOURCE_EXHAUSTED",
                    "details": [{ "@type": "type.googleapis.com/google.rpc.RetryInfo", "retryDelay": "13s" }]
                }
            })))
            .mount(&server)
            .await;

        let error = client(&server)
            .suggest_title("key-1", "gemini-3.8-flash", &frame_parts(), &[])
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            GeminiError::RateLimited { retry_after: Some(delay) } if delay == Duration::from_secs(13)
        ));
        assert!(error.is_transient());
    }

    #[tokio::test]
    async fn invalid_keys_and_missing_models_stop_the_queue() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "code": 400, "message": "API key not valid.", "status": "INVALID_ARGUMENT",
                    "details": [{ "@type": "type.googleapis.com/google.rpc.ErrorInfo", "reason": "API_KEY_INVALID" }]
                }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let key_error = client(&server).list_title_models("bad").await.unwrap_err();
        let model_error = client(&server)
            .suggest_title("key-1", "gemini-0-flash", &frame_parts(), &[])
            .await
            .unwrap_err();

        assert!(matches!(key_error, GeminiError::InvalidKey));
        assert!(
            matches!(&model_error, GeminiError::ModelNotFound(model) if model == "gemini-0-flash")
        );
        assert!(key_error.stops_queue() && model_error.stops_queue());
    }

    #[tokio::test]
    async fn depleted_credits_stop_the_queue_instead_of_failing_each_video() {
        let depleted = |status: u16| {
            ResponseTemplate::new(status).set_body_json(serde_json::json!({
                "error": {
                    "code": status, "status": "RESOURCE_EXHAUSTED",
                    "message": "Your prepayment credits are depleted. Please go to AI Studio."
                }
            }))
        };
        let current = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(depleted(402))
            .mount(&current)
            .await;
        let legacy = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(depleted(429))
            .mount(&legacy)
            .await;

        for server in [&current, &legacy] {
            let error = client(server)
                .suggest_title("key-1", "gemini-3.8-flash", &frame_parts(), &[])
                .await
                .unwrap_err();

            assert!(matches!(error, GeminiError::NoCredits));
            assert!(error.stops_queue() && !error.is_transient());
        }
    }

    #[tokio::test]
    async fn lists_only_flash_models_that_can_write_titles() {
        let server = MockServer::start().await;
        let methods = serde_json::json!(["generateContent", "countTokens"]);
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "models": [
                    { "name": "models/gemini-3.5-flash", "supportedGenerationMethods": methods },
                    { "name": "models/gemini-3.8-flash", "supportedGenerationMethods": methods },
                    { "name": "models/gemini-3.8-flash-tts", "supportedGenerationMethods": methods },
                    { "name": "models/gemini-3.1-flash-image", "supportedGenerationMethods": methods },
                    { "name": "models/gemini-3.8-pro", "supportedGenerationMethods": methods },
                    { "name": "models/gemini-3.1-flash-live", "supportedGenerationMethods": ["bidiGenerateContent"] }
                ]
            })))
            .mount(&server)
            .await;

        let models = client(&server).list_title_models("key-1").await.unwrap();

        assert_eq!(models, vec!["gemini-3.8-flash", "gemini-3.5-flash"]);
    }

    #[test]
    fn parses_retry_delays() {
        assert_eq!(parse_retry_delay("13s"), Some(Duration::from_secs(13)));
        assert_eq!(parse_retry_delay("1.5s"), Some(Duration::from_millis(1500)));
        assert_eq!(parse_retry_delay("soon"), None);
    }

    #[test]
    fn empty_answers_are_reported() {
        let response: GenerateResponse = serde_json::from_value(serde_json::json!({
            "candidates": [{ "content": { "parts": [] }, "finishReason": "SAFETY" }]
        }))
        .unwrap();

        let error = parse_suggestion(response).unwrap_err();

        assert!(matches!(error, GeminiError::BadResponse(message) if message.contains("SAFETY")));
    }
}
