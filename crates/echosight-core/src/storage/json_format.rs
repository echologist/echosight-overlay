use serde_json::Value;

/// JSON.stringify(value, null, 2) for JSON values: UTF-8, LF, no final newline,
/// insertion-ordered string keys after numerically sorted JS array-index keys.
/// Numbers are deliberately coerced to JS's f64 domain (not lossless Rust u64).
pub fn json_format(value: &Value) -> Result<String, serde_json::Error> {
    let mut output = String::new();
    append(value, 0, &mut output)?;
    Ok(output)
}

fn index(key: &str) -> Option<u32> {
    key.parse::<u32>()
        .ok()
        .filter(|&n| n != u32::MAX && n.to_string() == key)
}

fn indent(depth: usize, output: &mut String) {
    for _ in 0..depth {
        output.push_str("  ");
    }
}

fn append(value: &Value, depth: usize, output: &mut String) -> Result<(), serde_json::Error> {
    match value {
        Value::Number(number) => {
            output.push_str(ryu_js::Buffer::new().format(number.as_f64().unwrap()));
        }
        Value::Array(values) if !values.is_empty() => {
            output.push_str("[\n");
            for (i, value) in values.iter().enumerate() {
                indent(depth + 1, output);
                append(value, depth + 1, output)?;
                if i + 1 != values.len() {
                    output.push(',');
                }
                output.push('\n');
            }
            indent(depth, output);
            output.push(']');
        }
        Value::Object(values) if !values.is_empty() => {
            let mut entries: Vec<_> = values.iter().collect();
            // Stable sort leaves non-index properties in insertion order.
            entries.sort_by_key(|(key, _)| index(key).map_or((1, 0), |i| (0, i)));
            output.push_str("{\n");
            for (i, (key, value)) in entries.iter().enumerate() {
                indent(depth + 1, output);
                output.push_str(&serde_json::to_string(key)?);
                output.push_str(": ");
                append(value, depth + 1, output)?;
                if i + 1 != entries.len() {
                    output.push(',');
                }
                output.push('\n');
            }
            indent(depth, output);
            output.push('}');
        }
        _ => output.push_str(&serde_json::to_string(value)?),
    }
    Ok(())
}
