use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Invalid JSON: {0}")]
    Invalid(String),
    /// Valid destr output which cannot safely be represented by this Value API.
    /// A load must fail closed, not quarantine these inputs as corrupt.
    #[error("JSON outside the Value compatibility slice: {0}")]
    Unsupported(String),
}

static NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^-?[0-9]{1,16}(\.[0-9]{1,17})?([Ee][+-]?[0-9]+)?$").unwrap());
static SUSPECT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#""(?:(?:_|\\u0{2}5[Ff]){2}(?:p|\\u0{2}70)(?:r|\\u0{2}72)(?:o|\\u0{2}6[Ff])(?:t|\\u0{2}74)(?:o|\\u0{2}6[Ff])(?:_|\\u0{2}5[Ff]){2}|(?:c|\\u0063)(?:o|\\u006[Ff])(?:n|\\u006[Ee])(?:s|\\u0073)(?:t|\\u0074)(?:r|\\u0072)(?:u|\\u0075)(?:c|\\u0063)(?:t|\\u0074)(?:o|\\u006[Ff])(?:r|\\u0072))"[\t\n\r \u{a0}\u{feff}\u{2028}\u{2029}]*:"#).unwrap()
});

fn js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// Mirrors the installed destr strict parser for JSON-representable values.
/// Not a schema validator; even strings, booleans and null are accepted.
pub fn parse_json(text: &str) -> Result<Value, ParseError> {
    // destr's pre-trim fast path intentionally accepts unescaped embedded quotes
    // and controls. Replacing this with serde parsing would change existing data.
    if text.starts_with('"') && text.ends_with('"') && !text.contains('\\') {
        // JS slice(1, -1) returns empty even for the single-character string `"`.
        return Ok(Value::String(
            text.get(1..text.len() - 1).unwrap_or("").to_owned(),
        ));
    }
    let trimmed = text.trim_matches(js_space);
    match trimmed.to_ascii_lowercase().as_str() {
        "true" => return Ok(Value::Bool(true)),
        "false" => return Ok(Value::Bool(false)),
        "null" => return Ok(Value::Null),
        "undefined" | "nan" | "infinity" | "-infinity" => {
            return Err(ParseError::Unsupported(trimmed.to_owned()));
        }
        _ => {}
    }
    if !matches!(trimmed.chars().next(), Some('"' | '[' | '{')) && !NUMBER.is_match(trimmed) {
        return Err(ParseError::Invalid("[destr] Invalid JSON".into()));
    }
    if SUSPECT.is_match(text) {
        return Err(ParseError::Invalid(
            "[destr] Possible prototype pollution".into(),
        ));
    }
    let mut value: Value = serde_json::from_str(text).map_err(|error| {
        let message = error.to_string();
        if message.contains("recursion limit")
            || message.contains("surrogate")
            || message.contains("hex escape")
            || message.contains("number out of range")
        {
            ParseError::Unsupported(message)
        } else {
            ParseError::Invalid(message)
        }
    })?;
    js_numbers(&mut value);
    Ok(value)
}

fn js_numbers(value: &mut Value) {
    match value {
        Value::Array(values) => values.iter_mut().for_each(js_numbers),
        Value::Object(values) => values.values_mut().for_each(js_numbers),
        Value::Number(number)
            if number.as_u64().is_some_and(|n| n > 9_007_199_254_740_991)
                || number.as_i64().is_some_and(|n| n < -9_007_199_254_740_991) =>
        {
            *number = serde_json::Number::from_f64(number.as_f64().unwrap()).unwrap();
        }
        _ => {}
    }
}
