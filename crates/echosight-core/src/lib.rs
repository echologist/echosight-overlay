//! Echosight core: domain rules, persistence and theme loading.
//!
//! This crate must stay free of GPUI, FFI and platform code so that it can be
//! proven against golden fixtures generated from the TypeScript implementation.

pub mod window;
