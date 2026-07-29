use anyhow::{Result, bail};
use serde_json::{Number, Value};

pub fn substitute_path(
    template: &str,
    params: &Value,
    allowed_wildcard_prefixes: &[&str],
) -> Result<String> {
    let mut path = template.to_string();
    while let Some(start) = path.find('{') {
        let Some(end_offset) = path[start..].find('}') else {
            bail!("path template contains unmatched opening brace");
        };
        let end = start + end_offset;
        let key = &path[start + 1..end];
        if key.is_empty() {
            bail!("path template contains an empty parameter");
        }
        let value = path_scalar(params, key)?;
        path.replace_range(start..=end, &encode_path_segment(&value));
    }

    if path.contains("*path") {
        let Some(value) = params.get("path").and_then(Value::as_str) else {
            bail!("missing required path parameter: path");
        };
        validate_connector_path(value, allowed_wildcard_prefixes)?;
        path = path.replace("*path", value.trim_start_matches('/'));
    }

    Ok(path)
}

pub fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

pub fn validate_connector_path(path: &str, allowed_prefixes: &[&str]) -> Result<()> {
    if is_unsafe_connector_path(path) {
        bail!("unsafe connector path");
    }
    if allowed_prefixes
        .iter()
        .any(|prefix| path.starts_with(prefix))
    {
        Ok(())
    } else {
        bail!("connector path is outside the supported integration API prefix")
    }
}

fn path_scalar(params: &Value, key: &str) -> Result<String> {
    match params.get(key) {
        Some(Value::String(value)) => Ok(value.clone()),
        Some(Value::Number(value)) => Ok(number_to_string(value)),
        Some(Value::Bool(value)) => Ok(value.to_string()),
        Some(_) => bail!("path parameter {key} must be a string, number, or boolean"),
        None => bail!("missing required path parameter: {key}"),
    }
}

fn number_to_string(value: &Number) -> String {
    value
        .as_i64()
        .map(|v| v.to_string())
        .or_else(|| value.as_u64().map(|v| v.to_string()))
        .or_else(|| value.as_f64().map(|v| v.to_string()))
        .unwrap_or_else(|| value.to_string())
}

fn is_unsafe_connector_path(path: &str) -> bool {
    if path.is_empty()
        || !path.starts_with('/')
        || path.starts_with("//")
        || path.contains("://")
        || path.contains('\\')
        || path.contains('?')
        || path.contains('#')
        || path.contains("..")
    {
        return true;
    }

    let lowercase = path.to_ascii_lowercase();
    lowercase.contains("%2e")
        || lowercase.contains("%2f")
        || lowercase.contains("%5c")
        || lowercase.contains("%00")
}
