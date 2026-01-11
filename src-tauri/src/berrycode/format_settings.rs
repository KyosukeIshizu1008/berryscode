//! Settings formatting utilities

use crate::berrycode::Result;
use serde_json::Value;
use regex::Regex;

pub fn format_settings(settings: &Value) -> Result<String> {
    // Format settings as human-readable text
    let mut output = String::new();

    if let Value::Object(map) = settings {
        for (key, value) in map {
            let formatted_value = match value {
                Value::String(s) => {
                    // Scrub sensitive values
                    if is_sensitive_key(key) {
                        scrub_value(s)
                    } else {
                        s.clone()
                    }
                }
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => n.to_string(),
                Value::Array(arr) => format!("[{}]", arr.len()),
                Value::Object(_) => "{...}".to_string(),
                Value::Null => "null".to_string(),
            };
            output.push_str(&format!("{}: {}\n", key, formatted_value));
        }
    }

    Ok(output)
}

pub fn scrub_sensitive_info(text: &str) -> String {
    let mut scrubbed = text.to_string();

    // Scrub API keys
    let api_key_patterns = vec![
        r"sk-[a-zA-Z0-9]{48}",  // OpenAI API keys
        r"sk-ant-[a-zA-Z0-9\-]{95,}",  // Anthropic API keys
        r"Bearer [a-zA-Z0-9\-._~+/]+=*",  // Bearer tokens
        r#"api[_-]?key['"]?\s*[:=]\s*['"]?([a-zA-Z0-9\-._~+/]+)"#,  // Generic API keys
    ];

    for pattern in api_key_patterns {
        if let Ok(re) = Regex::new(pattern) {
            scrubbed = re.replace_all(&scrubbed, "***REDACTED***").to_string();
        }
    }

    // Scrub passwords
    let password_patterns = vec![
        r#"password['"]?\s*[:=]\s*['"]?([^\s'"]+)"#,
        r#"passwd['"]?\s*[:=]\s*['"]?([^\s'"]+)"#,
        r#"pwd['"]?\s*[:=]\s*['"]?([^\s'"]+)"#,
    ];

    for pattern in password_patterns {
        if let Ok(re) = Regex::new(pattern) {
            scrubbed = re.replace_all(&scrubbed, "password=***REDACTED***").to_string();
        }
    }

    // Scrub tokens
    let token_patterns = vec![
        r#"token['"]?\s*[:=]\s*['"]?([a-zA-Z0-9\-._~+/]{20,})"#,
        r#"access[_-]?token['"]?\s*[:=]\s*['"]?([a-zA-Z0-9\-._~+/]{20,})"#,
    ];

    for pattern in token_patterns {
        if let Ok(re) = Regex::new(pattern) {
            scrubbed = re.replace_all(&scrubbed, "token=***REDACTED***").to_string();
        }
    }

    scrubbed
}

fn is_sensitive_key(key: &str) -> bool {
    let sensitive_keys = vec![
        "api_key", "apikey", "api-key",
        "password", "passwd", "pwd",
        "token", "access_token", "access-token",
        "secret", "api_secret", "api-secret",
        "authorization", "auth",
    ];

    let key_lower = key.to_lowercase();
    sensitive_keys.iter().any(|&sk| key_lower.contains(sk))
}

fn scrub_value(value: &str) -> String {
    if value.len() <= 4 {
        "***".to_string()
    } else {
        format!("{}***", &value[..4])
    }
}
