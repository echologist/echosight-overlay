//! Settings defaults/migration and hotkey text/recording rules, ported from the
//! shipped TypeScript implementation. Explicit platform and event inputs keep
//! the domain usable by GPUI without a browser or global-hotkey dependency.
mod hotkeys;
mod model;
mod normalize;
mod recorder;

pub use hotkeys::*;
pub use model::*;
pub use normalize::*;
pub use recorder::*;
