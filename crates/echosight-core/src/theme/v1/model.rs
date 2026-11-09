//! A v1 theme is an open JSON object, not a closed typed schema.
//!
//! Only `id` and `name` are required by validation. Retaining JSON preserves
//! absent vs null, unknown fields and warning-only invalid variant definitions.
//! Always run `validate_theme` on raw `Value` before interpreting these fields.
//! serde_json's workspace `preserve_order` feature retains source property order.
pub type Theme = serde_json::Map<String, serde_json::Value>;
