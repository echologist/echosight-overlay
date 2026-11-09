//! Pure task domain. Source of truth: renderer/features/tasks/*.ts.
//! UI effects, trigger activation, expiry timers and persistence I/O are separate.
pub mod background;
pub mod completion;
pub mod creation;
pub mod migration;
pub mod model;
pub mod mutations;
pub mod progress;
pub mod state;
pub mod tree;
