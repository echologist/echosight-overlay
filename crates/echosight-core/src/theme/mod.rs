//! Pure v1 JSON validation and helpers, proven against the shipped TypeScript.
//! This is not a filesystem loader: discovery, sync and asset delivery remain pending.
pub mod helpers;
pub mod v1;
pub mod validate;
pub use helpers::*;
pub use validate::{Validation, ValidationMessages, validate_theme, validate_theme_variant_config};
