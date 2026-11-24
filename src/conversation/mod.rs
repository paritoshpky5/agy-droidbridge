use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct Message {
    role: Option<String>,
    #[serde(default)]
    content: Value,
    tool_calls: Option<Vec<Value>>,
    function_call: Option<Value>,
}

pub fn compile(
    messages: Vec<Message>,
    system: Option<Value>,
    anthropic: bool,
) -> Result<String, &'static str> {
    let mut prompt = String::new();
    if anthropic {
        if let Some(system) = system {
            let system = flatten(&system)?;
            if !system.is_empty() {
                append(&mut prompt, "System", &system);
            }
        }
    }
    for message in messages {
        if message.tool_calls.is_some_and(|calls| !calls.is_empty())
            || message.function_call.is_some()
        {
            return Err("Tool calling is unavailable. Send text messages.");
        }
        let role = match message
            .role
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "system" | "developer" if !anthropic => "System",
            "tool" if !anthropic => "Tool",
            "assistant" => "Assistant",
            _ => "User",
        };
        append(&mut prompt, role, &flatten(&message.content)?);
    }
    Ok(prompt)
}

fn append(prompt: &mut String, role: &str, content: &str) {
    if !prompt.is_empty() {
        prompt.push_str("\n\n");
    }
    prompt.push_str(role);
    prompt.push_str(": ");
    prompt.push_str(content);
}

fn flatten(content: &Value) -> Result<String, &'static str> {
    const TEXT_ONLY: &str = "Only text content is supported";
    if content.is_null() {
        return Ok(String::new());
    }
    if let Some(text) = content.as_str() {
        return Ok(text.into());
    }
    let parts = content.as_array().ok_or(TEXT_ONLY)?;
    let mut text = Vec::new();
    for part in parts {
        let block = if let Some(text) = part.as_str() {
            text
        } else if part
            .get("type")
            .is_none_or(|kind| kind.as_str() == Some("text"))
        {
            part.get("text").and_then(Value::as_str).ok_or(TEXT_ONLY)?
        } else {
            return Err(TEXT_ONLY);
        };
        if !block.is_empty() {
            text.push(block);
        }
    }
    Ok(text.join("\n"))
}

pub fn estimate_tokens(text: &str) -> u64 {
    text.encode_utf16().count().div_ceil(4) as u64
}
