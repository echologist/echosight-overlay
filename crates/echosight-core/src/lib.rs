//! Echosight core: domain rules, persistence and theme loading.
//!
//! This crate must stay free of GPUI, FFI and platform code so that it can be
//! proven against golden fixtures generated from the TypeScript implementation.

pub mod jscompat;
pub mod settings;
pub mod storage;
pub mod task;
pub mod template;
pub mod theme;
pub mod window;
