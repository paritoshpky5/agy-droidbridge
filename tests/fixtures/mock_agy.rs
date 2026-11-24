use std::{env, fs, process, thread, time::{Duration, SystemTime, UNIX_EPOCH}};

fn main() {
    let argv: Vec<_> = env::args().skip(1).collect();
    let prompt = argv.last().expect("prompt");
    let (json_mode, model) = match &argv[..] {
        [permission, print, _] if permission == "--dangerously-skip-permissions" && print == "-p" => (false, None),
        [flag, model, print, _] if flag == "--model" && model == "gemini-3.5-flash-low" && print == "--prompt" => (false, None),
        [agent, name, disable, format, json, print, _]
            if restricted(agent, name, disable, format, json) && print == "-p" => (true, None),
        [agent, name, disable, format, json, flag, model, print, _]
            if restricted(agent, name, disable, format, json) && flag == "--model" && print == "-p" => {
                (true, Some(model.as_str()))
            }
        _ => process::exit(90),
    };
    if json_mode {
        let agent = fs::read_to_string(".agents/agents/agy-droidbridge-text.md").unwrap();
        assert!(agent.lines().any(|line| line == "tools: []"));
        assert!(agent.lines().any(|line| line == "commandExecutionPolicy: off"));
        assert!(env::var_os("AGY_API_KEY").is_none());
        assert!(env::var_os("UNRELATED_PRIVATE_KEY").is_none());
    }
    if prompt.contains("MOCK_FAIL") || prompt.contains("MOCK_CRASH") {
        eprintln!("SECRET-FIXTURE /private/mock/settings");
        if prompt.contains("MOCK_CRASH") {
            process::abort();
        }
        process::exit(5);
    }
    if prompt.contains("MOCK_AUTH") || prompt.contains("MOCK_QUOTA") {
        eprintln!("SECRET-FIXTURE /private/mock/settings");
        if json_mode {
            println!(r#"{{"status":"ERROR","response":"","error":"SECRET-FIXTURE"}}"#);
        }
        return;
    }
    if prompt.contains("MOCK_EMPTY") {
        return;
    }
    if prompt.contains("MOCK_MALFORMED") {
        print!("not an envelope");
        return;
    }
    if prompt.contains("MOCK_LARGE") {
        print!("{}", "a".repeat(8192));
        return;
    }
    if prompt.contains("MOCK_TIMEOUT") {
        thread::sleep(Duration::from_millis(2600));
    }
    let text = if prompt.contains("MOCK_SERIAL") {
        let start = now();
        thread::sleep(Duration::from_millis(175));
        format!("{start}:{}", now())
    } else if prompt.contains("MOCK_MODEL") && json_mode {
        model.unwrap_or("default").to_owned()
    } else {
        format!("Mock reply: {prompt}")
    };
    if json_mode {
        println!(r#"{{"status":"SUCCESS","response":"{}"}}"#, escape_json(&text));
    } else {
        print!("{text}");
    }
}

fn restricted(agent: &str, name: &str, disable: &str, format: &str, json: &str) -> bool {
    (agent, name, disable, format, json)
        == ("--agent", "agy-droidbridge-text", "--disable-slash-commands", "--output-format", "json")
}

fn now() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
}

fn escape_json(text: &str) -> String {
    let mut result = String::new();
    for ch in text.chars() {
        match ch {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            ch if ch.is_control() => result.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => result.push(ch),
        }
    }
    result
}
