//! v1 JSON validation, explicit-root filesystem loading and compatibility delivery.
//! CSS is returned as data only; v2 conversion and GPUI rendering are separate.
pub mod assets;
pub mod library;
pub mod sounds;
pub mod sync;
pub use assets::ThemeRoot;
pub use library::{ThemeFailure, ThemeLibrary};
pub mod helpers;
pub mod v1;
pub mod validate;
pub use helpers::*;
pub use validate::{Validation, ValidationMessages, validate_theme, validate_theme_variant_config};
