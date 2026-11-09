//! Small JavaScript-semantics helpers used to keep the port byte-compatible
//! with the TypeScript implementation (`String.prototype.trim`, `Number(...)`,
//! `Math.round`, UTF-16 `substring`). Nothing here is Echosight-specific.

use serde_json::Value;

/// `String.prototype.trim()`: strips ECMAScript WhiteSpace and LineTerminator
/// code points (which differ slightly from Rust's `char::is_whitespace`).
pub fn js_trim(input: &str) -> &str {
    input.trim_matches(is_js_whitespace)
}

/// ECMAScript WhiteSpace + LineTerminator set.
pub fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// JavaScript's OrdinaryToPrimitive failure for a JSON object whose own
/// non-callable `toString` shadows Object.prototype.toString. JSON cannot carry
/// callable valueOf/toString overrides that could otherwise return a primitive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Cannot convert object to primitive value")]
pub struct JsNumberError;

/// `Number(value)` for arbitrary JSON, including failures while Array.join
/// converts nested elements. An error is NOT NaN: callers must not default/save.
pub fn checked_js_number(value: &Value) -> Result<f64, JsNumberError> {
    fn check_primitive_conversion(value: &Value) -> Result<(), JsNumberError> {
        match value {
            Value::Object(object) if object.contains_key("toString") => Err(JsNumberError),
            Value::Array(items) => {
                for item in items {
                    check_primitive_conversion(item)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
    check_primitive_conversion(value)?;
    Ok(js_number(value))
}

/// Infallible numeric conversion ONLY for JSON whose primitive conversion cannot
/// throw (no own `toString` on an object reached directly or through arrays).
/// Prefer `checked_js_number` at raw JSON boundaries. Retained for the legacy
/// task-creation slice, whose fixtures/typed UI options bound its supported
/// inputs; arbitrary raw task options remain a separate parity gate. Templates
/// validate the coerced field before calling that legacy sanitizer.
/// Arrays use Array.toString (join); `[true]` becomes "true", not the number 1.
pub fn js_number(value: &Value) -> f64 {
    match value {
        Value::Null => 0.0,
        Value::Bool(true) => 1.0,
        Value::Bool(false) => 0.0,
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => js_string_to_number(s),
        Value::Array(items) => js_string_to_number(&array_string(items)),
        Value::Object(_) => f64::NAN,
    }
}

fn array_string(items: &[Value]) -> String {
    items
        .iter()
        .map(|item| match item {
            Value::Null => String::new(),
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::Array(items) => array_string(items),
            Value::Object(_) => "[object Object]".into(),
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// `Number(string)` (StringToNumber): trimmed, empty is 0, decimal literals,
/// `Infinity`, and `0x`/`0o`/`0b` integer literals. Anything else is NaN.
pub fn js_string_to_number(input: &str) -> f64 {
    let trimmed = js_trim(input);
    if trimmed.is_empty() {
        return 0.0;
    }

    let (sign, body) = match trimmed.as_bytes()[0] {
        b'+' => (1.0, &trimmed[1..]),
        b'-' => (-1.0, &trimmed[1..]),
        _ => (1.0, trimmed),
    };

    if body == "Infinity" {
        return sign * f64::INFINITY;
    }

    if body.len() == trimmed.len() && body.len() > 2 && body.as_bytes()[0] == b'0' {
        let radix = match body.as_bytes()[1] {
            b'x' | b'X' => Some(16),
            b'o' | b'O' => Some(8),
            b'b' | b'B' => Some(2),
            _ => None,
        };
        if let Some(radix) = radix {
            return parse_radix(&body[2..], radix);
        }
    }

    if !is_decimal_literal(body) {
        return f64::NAN;
    }

    body.parse::<f64>().map(|v| sign * v).unwrap_or(f64::NAN)
}

fn parse_radix(digits: &str, radix: u32) -> f64 {
    if digits.is_empty() {
        return f64::NAN;
    }
    // Radices here are powers of two. Keep the leading 53 significant bits,
    // then a guard bit and sticky remainder; round ONCE, nearest/ties-to-even.
    // Iterative f64 multiply/add rounds too early (e.g. 0x200000000000011).
    let width = radix.ilog2();
    let mut length = 0u32;
    let mut significand = 0u64;
    let mut guard = false;
    let mut sticky = false;
    for c in digits.chars() {
        let Some(digit) = c.to_digit(radix) else {
            return f64::NAN;
        };
        for shift in (0..width).rev() {
            let bit = (digit >> shift) & 1;
            if length == 0 && bit == 0 {
                continue;
            }
            // Still validate every digit after overflow; a bad suffix is NaN.
            length = (length + 1).min(1025);
            match length {
                1..=53 => significand = (significand << 1) | u64::from(bit),
                54 => guard = bit != 0,
                _ => sticky |= bit != 0,
            }
        }
    }
    if length <= 53 {
        return significand as f64;
    }
    if length > 1024 {
        return f64::INFINITY;
    }
    if guard && (sticky || significand & 1 != 0) {
        significand += 1;
    }
    // Both factors are exact; carry out of the significand also handles the
    // max-finite/infinity midpoint correctly without intermediate overflow.
    significand as f64 * 2.0f64.powi(length as i32 - 53)
}

/// StrUnsignedDecimalLiteral: `digits[.digits][e[+-]digits]`, `.digits[e..]`.
fn is_decimal_literal(body: &str) -> bool {
    let bytes = body.as_bytes();
    let mut i = 0;
    let mut int_digits = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
        int_digits += 1;
    }
    let mut frac_digits = 0;
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            frac_digits += 1;
        }
    }
    if int_digits == 0 && frac_digits == 0 {
        return false;
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let mut exp_digits = 0;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            exp_digits += 1;
        }
        if exp_digits == 0 {
            return false;
        }
    }
    i == bytes.len()
}

/// `Math.round`: rounds half towards +Infinity (unlike `f64::round`).
pub fn js_round(value: f64) -> f64 {
    if !value.is_finite() {
        return value;
    }
    let floor = value.floor();
    if value - floor >= 0.5 {
        floor + 1.0
    } else {
        floor
    }
}

/// `Number.isInteger`.
pub fn js_is_integer(value: f64) -> bool {
    value.is_finite() && value.trunc() == value
}

/// `String.prototype.length` (UTF-16 code units).
pub fn utf16_len(input: &str) -> usize {
    input.encode_utf16().count()
}

/// `str.substring(0, max_units)` measured in UTF-16 code units. A surrogate
/// pair that would be split is dropped entirely (Rust strings cannot hold a
/// lone surrogate; the TypeScript side would emit one, which JSON then
/// serialises as `\ud83d`). Documented as a known, unreachable-in-practice gap.
pub fn utf16_prefix(input: &str, max_units: usize) -> String {
    let mut units = 0;
    let mut end = 0;
    for (index, c) in input.char_indices() {
        let width = c.len_utf16();
        if units + width > max_units {
            break;
        }
        units += width;
        end = index + c.len_utf8();
    }
    input[..end].to_string()
}

/// Truthiness of a JSON value (`!!value`).
pub fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|v| v != 0.0 && !v.is_nan()).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// `isRecord` from `taskGuards.ts`: a non-null, non-array object.
pub fn is_record(value: &Value) -> bool {
    value.is_object()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn trim_uses_ecmascript_whitespace() {
        assert_eq!(js_trim("\u{FEFF} a \u{00A0}"), "a");
        assert_eq!(js_trim("\u{0085}a"), "\u{0085}a");
    }

    #[test]
    fn number_coercion_matches_js() {
        assert_eq!(js_number(&json!(null)), 0.0);
        assert_eq!(js_number(&json!(true)), 1.0);
        assert_eq!(js_number(&json!("  78.4 ")), 78.4);
        assert_eq!(js_number(&json!("")), 0.0);
        assert!(js_number(&json!("abc")).is_nan());
        assert!(js_number(&json!("1e")).is_nan());
        assert_eq!(js_number(&json!("0x1f")), 31.0);
        assert_eq!(js_number(&json!(".5")), 0.5);
        assert_eq!(js_number(&json!("5.")), 5.0);
        assert_eq!(js_number(&json!("-Infinity")), f64::NEG_INFINITY);
        assert!(js_number(&json!("infinity")).is_nan());
        assert!(js_number(&json!({})).is_nan());
        assert_eq!(js_number(&json!([7])), 7.0);
    }

    #[test]
    fn round_half_up() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(0.49999999999999994), 0.0);
        assert_eq!(js_round(78.4), 78.0);
    }

    #[test]
    fn utf16_prefix_counts_code_units() {
        assert_eq!(utf16_prefix("abc", 2), "ab");
        assert_eq!(utf16_len("a😀"), 3);
        assert_eq!(utf16_prefix("a😀b", 2), "a");
        assert_eq!(utf16_prefix("a😀b", 3), "a😀");
    }
}
