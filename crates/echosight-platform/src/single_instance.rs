//! Single-instance lock with a "reveal" channel, replacing Electron's
//! `app.requestSingleInstanceLock()` + `second-instance` event
//! (`src/main/lifecycle/singleInstance.ts`, inventory section 6).
//!
//! Semantics:
//! - The first process to call [`SingleInstance::acquire`] becomes the
//!   primary, owns a [`SingleInstanceGuard`] and gets `on_reveal` invoked (on a
//!   background thread) every time another instance starts.
//! - A later process gets [`SingleInstanceError::AlreadyRunning`] after it has
//!   signalled the primary; the caller is expected to exit without doing any
//!   further setup (nothing else runs when the lock is denied).
//! - Dropping the guard releases the lock so a new primary can start.
//!
//! Transport:
//! - **Windows**: a named mutex `Local\<app_id>.SingleInstance` decides
//!   ownership (`ERROR_ALREADY_EXISTS` = secondary) and a named pipe
//!   `\\.\pipe\<app_id lowercase>-reveal` carries the reveal signal: the
//!   primary runs a listener thread that blocks in `ConnectNamedPipe`, the
//!   secondary simply opens the pipe with `CreateFileW` and closes it.
//! - **Unix**: a Unix domain socket at `$XDG_RUNTIME_DIR/<app_id lowercase>.sock`
//!   (fallback [`std::env::temp_dir`]). `bind` success = primary; if the path
//!   is busy the process connects (= reveal) and reports `AlreadyRunning`; if
//!   the path is stale (connect refused) it is unlinked and bound again.
//!
//! Only the Windows backend contains `unsafe` (Win32 calls), each annotated.

use std::fmt;

/// App id used by the overlay; produces `Local\Echosight.SingleInstance` and
/// `\\.\pipe\echosight-reveal` on Windows, `echosight.sock` on Unix.
pub const ECHOSIGHT_APP_ID: &str = "Echosight";

#[derive(Debug, thiserror::Error)]
pub enum SingleInstanceError {
    /// Another instance holds the lock; it has been asked to reveal itself.
    #[error("another instance is already running")]
    AlreadyRunning,
    #[error("single instance I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[cfg(windows)]
    #[error("single instance Win32 error: {0}")]
    Win32(#[from] windows::core::Error),
}

/// Callback invoked by the primary when a second instance starts.
pub type RevealCallback = Box<dyn Fn() + Send + Sync + 'static>;

/// Namespace for the lock API.
pub struct SingleInstance;

impl SingleInstance {
    /// Tries to become the primary instance for `app_id`.
    pub fn acquire<F>(
        app_id: &str,
        on_reveal: F,
    ) -> Result<SingleInstanceGuard, SingleInstanceError>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let inner = imp::acquire(app_id, Box::new(on_reveal))?;
        Ok(SingleInstanceGuard { _inner: inner })
    }

    /// Human-readable description of the OS resource used for `app_id`.
    pub fn resource_name(app_id: &str) -> String {
        imp::resource_name(app_id)
    }
}

/// Held by the primary for the lifetime of the app. Dropping it releases the lock.
pub struct SingleInstanceGuard {
    _inner: imp::Guard,
}

impl fmt::Debug for SingleInstanceGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SingleInstanceGuard")
            .finish_non_exhaustive()
    }
}

#[cfg(unix)]
mod imp {
    use super::{RevealCallback, SingleInstanceError};
    use std::io::{self, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread::{self, JoinHandle};

    pub fn socket_path(app_id: &str) -> PathBuf {
        let dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
            .unwrap_or_else(std::env::temp_dir);
        dir.join(format!("{}.sock", app_id.to_lowercase()))
    }

    pub fn resource_name(app_id: &str) -> String {
        socket_path(app_id).display().to_string()
    }

    pub struct Guard {
        path: PathBuf,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    pub fn acquire(app_id: &str, on_reveal: RevealCallback) -> Result<Guard, SingleInstanceError> {
        let path = socket_path(app_id);
        let listener = match UnixListener::bind(&path) {
            Ok(listener) => listener,
            Err(err) if err.kind() == io::ErrorKind::AddrInUse => {
                match UnixStream::connect(&path) {
                    Ok(mut stream) => {
                        // Primary is alive: ask it to reveal and give up.
                        let _ = stream.write_all(b"reveal\n");
                        return Err(SingleInstanceError::AlreadyRunning);
                    }
                    Err(connect_err) => {
                        log::info!(
                            "removing stale single-instance socket {} ({connect_err})",
                            path.display()
                        );
                        std::fs::remove_file(&path)?;
                        UnixListener::bind(&path)?
                    }
                }
            }
            Err(err) => return Err(err.into()),
        };

        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name("echosight-single-instance".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    if thread_stop.load(Ordering::SeqCst) {
                        break;
                    }
                    match stream {
                        Ok(_) => on_reveal(),
                        Err(err) => {
                            log::warn!("single-instance accept failed: {err}");
                            break;
                        }
                    }
                }
            })?;

        Ok(Guard {
            path,
            stop,
            thread: Some(thread),
        })
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            // Wake the blocking `accept` so the thread observes `stop`.
            let _ = UnixStream::connect(&self.path);
            let _ = std::fs::remove_file(&self.path);
            if let Some(thread) = self.thread.take()
                && thread.join().is_err()
            {
                log::error!("single-instance listener thread panicked");
            }
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::{RevealCallback, SingleInstanceError};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread::{self, JoinHandle};
    use windows::Win32::Foundation::{
        CloseHandle, ERROR_ALREADY_EXISTS, ERROR_PIPE_CONNECTED, GENERIC_WRITE, GetLastError,
        HANDLE,
    };
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_NONE, OPEN_EXISTING, PIPE_ACCESS_INBOUND,
    };
    use windows::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE,
        PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
    };
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::{HRESULT, HSTRING};

    pub fn mutex_name(app_id: &str) -> String {
        format!("Local\\{app_id}.SingleInstance")
    }

    pub fn pipe_name(app_id: &str) -> String {
        format!("\\\\.\\pipe\\{}-reveal", app_id.to_lowercase())
    }

    pub fn resource_name(app_id: &str) -> String {
        format!("{} + {}", mutex_name(app_id), pipe_name(app_id))
    }

    /// Owned Win32 handle. Handles are process-wide kernel object references,
    /// so moving one across threads is sound.
    struct OwnedHandle(HANDLE);

    // SAFETY: a HANDLE is an opaque kernel object reference without thread
    // affinity; only `CloseHandle` is ever called on it, exactly once, in Drop.
    unsafe impl Send for OwnedHandle {}
    // SAFETY: see above; shared access only reads the handle value.
    unsafe impl Sync for OwnedHandle {}

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            if !self.0.is_invalid() {
                // SAFETY: the handle was returned by a successful Win32 create
                // call and has not been closed before.
                let _ = unsafe { CloseHandle(self.0) };
            }
        }
    }

    pub struct Guard {
        _mutex: OwnedHandle,
        pipe_name: HSTRING,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    pub fn acquire(app_id: &str, on_reveal: RevealCallback) -> Result<Guard, SingleInstanceError> {
        let mutex_name = HSTRING::from(mutex_name(app_id));
        let pipe = HSTRING::from(pipe_name(app_id));

        // SAFETY: `mutex_name` is a valid, NUL-terminated wide string that
        // outlives the call; no security attributes are passed.
        let handle = unsafe { CreateMutexW(None, false, &mutex_name)? };
        let mutex = OwnedHandle(handle);
        // SAFETY: plain thread-local error read, immediately after CreateMutexW.
        let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        if already_exists {
            drop(mutex);
            signal_primary(&pipe);
            return Err(SingleInstanceError::AlreadyRunning);
        }

        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let thread_pipe = pipe.clone();
        let thread = thread::Builder::new()
            .name("echosight-single-instance".into())
            .spawn(move || listen(&thread_pipe, &thread_stop, on_reveal))?;

        Ok(Guard {
            _mutex: mutex,
            pipe_name: pipe,
            stop,
            thread: Some(thread),
        })
    }

    /// Secondary side: opening the pipe is the whole protocol.
    fn signal_primary(pipe: &HSTRING) {
        // SAFETY: `pipe` is a valid wide string; the returned handle is closed
        // right away by `OwnedHandle`.
        let opened = unsafe {
            CreateFileW(
                pipe,
                GENERIC_WRITE.0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        };
        match opened {
            Ok(handle) => drop(OwnedHandle(handle)),
            Err(err) => log::warn!("could not signal the running Echosight instance: {err}"),
        }
    }

    /// Primary side: one pipe instance at a time, accept, notify, disconnect, repeat.
    fn listen(pipe: &HSTRING, stop: &AtomicBool, on_reveal: RevealCallback) {
        let pipe_connected = HRESULT::from_win32(ERROR_PIPE_CONNECTED.0);
        while !stop.load(Ordering::SeqCst) {
            // SAFETY: `pipe` is a valid wide string; all other arguments are
            // plain flags/sizes and no security attributes are passed.
            let handle = unsafe {
                CreateNamedPipeW(
                    pipe,
                    PIPE_ACCESS_INBOUND,
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                    1,
                    64,
                    64,
                    0,
                    None,
                )
            };
            if handle.is_invalid() {
                log::warn!(
                    "CreateNamedPipeW failed: {}",
                    windows::core::Error::from_thread()
                );
                return;
            }
            let instance = OwnedHandle(handle);

            // SAFETY: `instance.0` is a live pipe handle created above; a
            // synchronous (non-overlapped) connect is requested.
            let connected = match unsafe { ConnectNamedPipe(instance.0, None) } {
                Ok(()) => true,
                // Client raced us between Create and Connect: still connected.
                Err(err) if err.code() == pipe_connected => true,
                Err(err) => {
                    log::warn!("ConnectNamedPipe failed: {err}");
                    false
                }
            };
            // SAFETY: same live handle; disconnecting an unconnected instance
            // only returns an error we ignore.
            let _ = unsafe { DisconnectNamedPipe(instance.0) };
            drop(instance);

            if connected && !stop.load(Ordering::SeqCst) {
                on_reveal();
            }
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            // Wake the listener blocked in ConnectNamedPipe by connecting once.
            signal_primary(&self.pipe_name);
            if let Some(thread) = self.thread.take()
                && thread.join().is_err()
            {
                log::error!("single-instance listener thread panicked");
            }
            // `_mutex` is released by its Drop (CloseHandle) after the thread is gone.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    fn unique_app_id(tag: &str) -> String {
        format!("EchosightTest-{tag}-{}", std::process::id())
    }

    fn wait_for(counter: &AtomicUsize, expected: usize) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while counter.load(Ordering::SeqCst) < expected && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        counter.load(Ordering::SeqCst) >= expected
    }

    #[test]
    fn second_acquire_in_same_process_fails_and_reveals_primary() {
        let app_id = unique_app_id("same-process");
        let reveals = Arc::new(AtomicUsize::new(0));
        let primary_reveals = Arc::clone(&reveals);
        let primary = SingleInstance::acquire(&app_id, move || {
            primary_reveals.fetch_add(1, Ordering::SeqCst);
        })
        .expect("first acquire is the primary");

        let second =
            SingleInstance::acquire(&app_id, || panic!("secondary must never be revealed"));
        assert!(matches!(second, Err(SingleInstanceError::AlreadyRunning)));
        assert!(
            wait_for(&reveals, 1),
            "primary reveal callback did not fire"
        );

        let third = SingleInstance::acquire(&app_id, || {});
        assert!(matches!(third, Err(SingleInstanceError::AlreadyRunning)));
        assert!(wait_for(&reveals, 2), "second reveal did not fire");

        drop(primary);
        // Lock released: a new primary can start.
        let again = SingleInstance::acquire(&app_id, || {}).expect("re-acquire after drop");
        assert_eq!(
            reveals.load(Ordering::SeqCst),
            2,
            "drop wake-up must not count as a reveal"
        );
        drop(again);
    }

    /// Child-process variant: the secondary is a separate OS process, exactly
    /// like a user double-clicking the exe twice. The test binary re-executes
    /// itself with `ECHOSIGHT_SI_CHILD=<app id>` and the child's exit code is
    /// the verdict (3 = AlreadyRunning, 0 = it wrongly became the primary).
    #[test]
    fn second_process_fails_and_reveals_primary() {
        const CHILD_ENV: &str = "ECHOSIGHT_SI_CHILD";
        if let Some(app_id) = std::env::var_os(CHILD_ENV) {
            let code = match SingleInstance::acquire(&app_id.to_string_lossy(), || {}) {
                Ok(_guard) => 0,
                Err(SingleInstanceError::AlreadyRunning) => 3,
                Err(_) => 4,
            };
            std::process::exit(code);
        }

        let app_id = unique_app_id("child-process");
        let reveals = Arc::new(AtomicUsize::new(0));
        let primary_reveals = Arc::clone(&reveals);
        let _primary = SingleInstance::acquire(&app_id, move || {
            primary_reveals.fetch_add(1, Ordering::SeqCst);
        })
        .expect("primary");

        let exe = std::env::current_exe().expect("current_exe");
        let status = std::process::Command::new(exe)
            .arg("single_instance::tests::second_process_fails_and_reveals_primary")
            .arg("--exact")
            .arg("--nocapture")
            .env(CHILD_ENV, &app_id)
            .status()
            .expect("spawn child test process");
        assert_eq!(
            status.code(),
            Some(3),
            "child must exit with 3 (AlreadyRunning)"
        );
        assert!(wait_for(&reveals, 1), "child did not reveal the primary");
    }

    #[test]
    fn resource_names_follow_the_plan() {
        let name = SingleInstance::resource_name(ECHOSIGHT_APP_ID);
        #[cfg(windows)]
        assert_eq!(
            name,
            "Local\\Echosight.SingleInstance + \\\\.\\pipe\\echosight-reveal"
        );
        #[cfg(unix)]
        assert!(name.ends_with("/echosight.sock"), "{name}");
    }
}
