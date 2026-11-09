//! Error type shared by all platform operations.

/// Errors from platform-specific window and hotkey operations.
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    /// GPUI could not hand out a raw window handle for this window.
    #[error("raw window handle unavailable: {0}")]
    HandleUnavailable(#[from] raw_window_handle::HandleError),

    /// The raw handle was not the kind expected on this platform (e.g. not `Win32`).
    #[error("unexpected raw window handle kind for this platform: {0}")]
    UnexpectedHandleKind(&'static str),

    /// The operation is not implemented on this platform (yet).
    #[error("{op} is unsupported on this platform")]
    Unsupported {
        /// Name of the operation that was attempted.
        op: &'static str,
    },

    /// An operating-system call failed.
    #[error("{op} failed: {source}")]
    Os {
        /// Name of the OS call that failed.
        op: &'static str,
        /// The underlying OS error.
        #[source]
        source: std::io::Error,
    },
}
