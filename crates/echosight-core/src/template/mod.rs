//! Template domain parity with taskTemplates.ts and templateOperations.ts.
//! No filesystem pickers, UI state, timers, history, or global clock/entropy.
//! Normalize untrusted saved data before decoding/using typed template records.
pub mod community;
pub mod model;
pub mod normalize;
pub mod ops;
pub mod persistence;
pub mod transfer;
