use crate::{cli, conversation, models, security};
use axum::body::{Body, to_bytes};
use axum::extract::{Request, State};
use axum::http::{Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};
use std::error::Error as _;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

struct App {
    cli: cli::Executor,
    models: models::Catalog,
    key: Option<String>,
    max_body: usize,
    max_prompt: usize,
}

pub fn router(
    cli: cli::Executor,
    models: models::Catalog,
    key: Option<String>,
    max_body: usize,
    max_prompt: usize,
) -> Router {
    Router::new().fallback(dispatch).with_state(Arc::new(App {
        cli,
        models,
        key,
        max_body,
        max_prompt,
    }))
}

async fn dispatch(State(state): State<Arc<App>>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let is_anthropic = parts.method == Method::POST && parts.uri.path() == "/v1/messages";
    let protocol = Protocol {
        anthropic: is_anthropic,
    };
    let generation = parts.method == Method::POST
        && matches!(parts.uri.path(), "/v1/chat/completions" | "/v1/messages");
    let list_models = parts.method == Method::GET && parts.uri.path() == "/v1/models";
    if parts.method == Method::OPTIONS {
        return StatusCode::NO_CONTENT.into_response();
    }
    if parts.method == Method::GET && matches!(parts.uri.path(), "/" | "/healthz") {
        return Json(json!({ "ok": true, "service": "agy-droidbridge" })).into_response();
    }
    if !(generation || list_models) {
        return protocol.error(
            StatusCode::NOT_FOUND,
            "No route for this request",
            "not_found",
        );
    }
    if !security::permits_origin(&parts.headers, &parts.uri) {
        return protocol.error(
            StatusCode::FORBIDDEN,
            "Only local API clients are allowed",
            "permission_error",
        );
    }
    if !security::authenticates(&parts.headers, state.key.as_deref(), is_anthropic) {
        return protocol.error(
            StatusCode::UNAUTHORIZED,
            "Invalid API key",
            "authentication_error",
        );
    }
    if list_models {
        let created = timestamp();
        let data: Vec<_> = state
            .models
            .0
            .iter()
            .map(|model| {
                json!({
                    "id": model.id, "label": model.label, "object": "model",
                    "created": created, "owned_by": model.owner
                })
            })
            .collect();
        return Json(json!({ "object": "list", "data": data })).into_response();
    }
    generate(&state, body, protocol).await
}

#[derive(Default, Deserialize)]
struct CompletionRequest {
    model: Option<String>,
    messages: Option<Vec<conversation::Message>>,
    #[serde(default)]
    stream: bool,
    stream_options: Option<StreamOptions>,
    system: Option<Value>,
    tools: Option<Vec<Value>>,
    functions: Option<Vec<Value>>,
    n: Option<u64>,
    response_format: Option<Value>,
}

#[derive(Deserialize)]
struct StreamOptions {
    #[serde(default)]
    include_usage: bool,
}

#[derive(Clone, Copy)]
struct Protocol {
    anthropic: bool,
}

impl Protocol {
    fn error(self, status: StatusCode, message: &str, kind: &str) -> Response {
        let mut body = json!({ "error": { "message": message, "type": kind } });
        if self.anthropic {
            body["type"] = json!("error");
        }
        let mut response = (status, Json(body)).into_response();
        if status == StatusCode::UNAUTHORIZED && !self.anthropic {
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, "Bearer".parse().unwrap());
        }
        response
    }

    fn invalid(self, message: &str) -> Response {
        self.error(StatusCode::BAD_REQUEST, message, "invalid_request_error")
    }

    fn upstream(self, error: cli::Error) -> Response {
        let (status, kind) = match error {
            cli::Error::Busy => (StatusCode::SERVICE_UNAVAILABLE, "overloaded_error"),
            cli::Error::Missing | cli::Error::Spawn => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_server_error")
            }
            cli::Error::Timeout => (StatusCode::GATEWAY_TIMEOUT, "upstream_error"),
            _ => (StatusCode::BAD_GATEWAY, "upstream_error"),
        };
        tracing::warn!(status = status.as_u16(), reason = %error, "generation failed");
        self.error(status, &error.to_string(), kind)
    }
}

async fn generate(state: &App, body: Body, protocol: Protocol) -> Response {
    let body = match to_bytes(body, state.max_body).await {
        Ok(body) => body,
        Err(error) => {
            if error
                .source()
                .is_some_and(|source| source.is::<http_body_util::LengthLimitError>())
            {
                return protocol.error(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "Request body is too large",
                    "invalid_request_error",
                );
            }
            return protocol.invalid("Invalid request body");
        }
    };
    let request: CompletionRequest = if body.iter().all(u8::is_ascii_whitespace) {
        CompletionRequest::default()
    } else {
        match serde_json::from_slice(&body) {
            Ok(request) => request,
            Err(_) => return protocol.invalid("Invalid JSON body"),
        }
    };
    if protocol.anthropic && request.stream {
        return protocol.invalid("Streaming is not supported by this proxy");
    }
    if request.tools.is_some_and(|tools| !tools.is_empty())
        || request
            .functions
            .is_some_and(|functions| !functions.is_empty())
    {
        return protocol.invalid("Tool calling is unavailable. Send a text request.");
    }
    if request.n.is_some_and(|count| count != 1) {
        return protocol.invalid("Only one completion per request is supported");
    }
    if request
        .response_format
        .is_some_and(|format| format.get("type").and_then(Value::as_str) != Some("text"))
    {
        return protocol.invalid("Only text responses are supported");
    }
    let Some(messages) = request.messages.filter(|messages| !messages.is_empty()) else {
        return protocol.invalid("`messages` is required and must be non-empty");
    };
    let model = match request.model.as_deref().filter(|model| !model.is_empty()) {
        None => None,
        Some(model) => match state.models.resolve(model) {
            Some(model) => Some(model),
            None => return protocol.invalid("Unknown model. Use GET /v1/models to list models."),
        },
    };
    let prompt = match conversation::compile(messages, request.system, protocol.anthropic) {
        Ok(prompt) => prompt,
        Err(error) => return protocol.invalid(error),
    };
    if prompt.len() > state.max_prompt {
        return protocol.error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Compiled prompt is too large",
            "invalid_request_error",
        );
    }
    if prompt.contains('\0') {
        return protocol.invalid("Messages cannot contain NUL characters");
    }
    let estimate = conversation::estimate_tokens(&prompt);
    let output = match state
        .cli
        .run(prompt, model.map(|model| model.id.clone()))
        .await
    {
        Ok(output) => output,
        Err(error) => return protocol.upstream(error),
    };
    let input_tokens = output.input_tokens.unwrap_or(estimate);
    let output_tokens = output
        .output_tokens
        .unwrap_or_else(|| conversation::estimate_tokens(&output.text));
    let model = model.map_or("", |model| model.label.as_str());
    if protocol.anthropic {
        return Json(json!({
            "id": format!("msg_{}", Uuid::new_v4().simple()), "type": "message", "role": "assistant",
            "model": model, "content": [{ "type": "text", "text": output.text }],
            "stop_reason": "end_turn", "stop_sequence": null,
            "usage": { "input_tokens": input_tokens, "output_tokens": output_tokens }
        })).into_response();
    }
    let id = format!("chatcmpl-{}", Uuid::new_v4());
    let created = timestamp();
    let usage = json!({
        "prompt_tokens": input_tokens, "completion_tokens": output_tokens,
        "total_tokens": input_tokens.saturating_add(output_tokens)
    });
    if request.stream {
        return stream(
            &id,
            created,
            model,
            output.text,
            usage,
            request
                .stream_options
                .is_some_and(|options| options.include_usage),
        );
    }
    Json(json!({
        "id": id, "object": "chat.completion", "created": created, "model": model,
        "choices": [{ "index": 0, "message": { "role": "assistant", "content": output.text }, "finish_reason": "stop" }],
        "usage": usage
    })).into_response()
}

fn stream(
    id: &str,
    created: u64,
    model: &str,
    text: String,
    usage: Value,
    include_usage: bool,
) -> Response {
    let mut events = String::new();
    let mut push = |choices: Value, usage: Option<Value>| {
        use std::fmt::Write as _;
        let mut chunk = json!({
            "id": id, "object": "chat.completion.chunk", "created": created, "model": model, "choices": choices
        });
        if include_usage {
            chunk["usage"] = usage.unwrap_or(Value::Null);
        }
        writeln!(events, "data: {chunk}\n").expect("writing to String cannot fail");
    };
    push(
        json!([{ "index": 0, "delta": { "role": "assistant", "content": "" }, "finish_reason": null }]),
        None,
    );
    push(
        json!([{ "index": 0, "delta": { "content": text }, "finish_reason": null }]),
        None,
    );
    push(
        json!([{ "index": 0, "delta": {}, "finish_reason": "stop" }]),
        None,
    );
    if include_usage {
        push(json!([]), Some(usage));
    }
    events.push_str("data: [DONE]\n\n");
    (
        [
            (header::CONTENT_TYPE, "text/event-stream; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        events,
    )
        .into_response()
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
