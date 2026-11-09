//! Path of Exile process monitor, ported from `src/main/game/poe2Monitor.ts`.
//!
//! Every `interval` (3 s in production) the process list is scanned for a
//! process whose name or executable file name normalises to contain
//! `pathofexile` (see [`crate::is_path_of_exile_process`]). Edge semantics are
//! identical to the TypeScript monitor:
//!
//! | previous | now     | event          |
//! |----------|---------|----------------|
//! | stopped  | running | `Started`      |
//! | running  | running | `StillRunning` |
//! | running  | stopped | `Stopped` once |
//! | stopped  | stopped | nothing        |
//!
//! The first check happens immediately on `start` (so a game that is already
//! running produces `Started` on the first tick), a failed process query is
//! logged and leaves the state untouched, and `stop()` resets the state so a
//! later `start()` reports `Started` again.
//!
//! The process source is injectable ([`ProcessSource`]) so the edge logic is
//! unit-tested without `sysinfo`; [`SysinfoSource`] is the production source.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::is_path_of_exile_process;

/// Production poll interval (`DEFAULT_INTERVAL_MS = 3000`).
pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEvent {
    /// The game appeared since the previous tick (or was running on the first tick).
    Started,
    /// The game was running on the previous tick and is gone now.
    Stopped,
    /// The game was running on the previous tick and still is.
    StillRunning,
}

#[derive(Debug, thiserror::Error)]
pub enum GameMonitorError {
    #[error("failed to initialise sysinfo: {0}")]
    Sysinfo(String),
    #[error("failed to query running processes: {0}")]
    Query(String),
    #[error("failed to spawn the game monitor thread: {0}")]
    Spawn(#[from] std::io::Error),
}

/// Something that can answer "is the game running right now?".
pub trait ProcessSource: Send + 'static {
    fn game_running(&mut self) -> Result<bool, GameMonitorError>;
}

/// Any closure returning the current running state works as a source.
impl<F> ProcessSource for F
where
    F: FnMut() -> Result<bool, GameMonitorError> + Send + 'static,
{
    fn game_running(&mut self) -> Result<bool, GameMonitorError> {
        self()
    }
}

/// Pure edge detector shared by the thread and the unit tests.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct EdgeDetector {
    running: bool,
}

impl EdgeDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds one observation and returns the event to emit, if any.
    pub fn observe(&mut self, running_now: bool) -> Option<GameEvent> {
        let was_running = self.running;
        self.running = running_now;
        match (was_running, running_now) {
            (false, true) => Some(GameEvent::Started),
            (true, false) => Some(GameEvent::Stopped),
            (true, true) => Some(GameEvent::StillRunning),
            (false, false) => None,
        }
    }

    /// `stop()` semantics: forget that the game was running.
    pub fn reset(&mut self) {
        self.running = false;
    }

    pub fn is_running(&self) -> bool {
        self.running
    }
}

/// Background-thread monitor. Dropping it stops the thread.
pub struct GameMonitor {
    stop_tx: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl GameMonitor {
    /// Starts polling with the production [`SysinfoSource`].
    pub fn start<F>(interval: Duration, on_event: F) -> Result<Self, GameMonitorError>
    where
        F: FnMut(GameEvent) + Send + 'static,
    {
        Self::start_with_source(interval, SysinfoSource::new()?, on_event)
    }

    /// Starts polling with a custom source (tests, fakes, other matchers).
    pub fn start_with_source<S, F>(
        interval: Duration,
        mut source: S,
        mut on_event: F,
    ) -> Result<Self, GameMonitorError>
    where
        S: ProcessSource,
        F: FnMut(GameEvent) + Send + 'static,
    {
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let thread = thread::Builder::new()
            .name("echosight-game-monitor".into())
            .spawn(move || {
                let mut edges = EdgeDetector::new();
                loop {
                    match source.game_running() {
                        Ok(running) => {
                            if let Some(event) = edges.observe(running) {
                                on_event(event);
                            }
                        }
                        Err(err) => log::error!("{err}"),
                    }
                    match stop_rx.recv_timeout(interval) {
                        Err(RecvTimeoutError::Timeout) => continue,
                        // Explicit stop or the monitor handle was dropped.
                        Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
            })?;
        Ok(Self {
            stop_tx: Some(stop_tx),
            thread: Some(thread),
        })
    }

    /// Stops polling and waits for the thread. No `Stopped` event is emitted
    /// (same as `poe2Monitor.stop()`), and a later `start` begins from the
    /// "not running" state.
    pub fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            log::error!("game monitor thread panicked");
        }
    }
}

impl Drop for GameMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Minimal `sysinfo` refresh: process names and executable paths only, no
/// CPU/memory/disk counters and no Linux task (thread) enumeration.
fn refresh_kind() -> ProcessRefreshKind {
    ProcessRefreshKind::nothing()
        .without_tasks()
        .with_exe(UpdateKind::OnlyIfNotSet)
}

/// Returns true if either the process name or its executable file name is a
/// Path of Exile binary.
fn process_matches(process: &sysinfo::Process) -> bool {
    if is_path_of_exile_process(&process.name().to_string_lossy()) {
        return true;
    }
    process
        .exe()
        .and_then(|p| p.file_name())
        .is_some_and(|f| is_path_of_exile_process(&f.to_string_lossy()))
}

/// Production source backed by [`sysinfo::System`]. The `System` is kept
/// between ticks so sysinfo can diff instead of re-reading everything.
pub struct SysinfoSource {
    system: System,
}

impl SysinfoSource {
    pub fn new() -> Result<Self, GameMonitorError> {
        let system = System::new().map_err(|e| GameMonitorError::Sysinfo(e.to_string()))?;
        Ok(Self { system })
    }

    fn refresh(&mut self) {
        self.system
            .refresh_processes_specifics(ProcessesToUpdate::All, true, refresh_kind());
    }

    /// Names of all processes that match, for diagnostics.
    pub fn matching_names(&mut self) -> Vec<String> {
        self.refresh();
        let mut names: Vec<String> = self
            .system
            .processes()
            .values()
            .filter(|p| process_matches(p))
            .map(|p| p.name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names.dedup();
        names
    }
}

impl ProcessSource for SysinfoSource {
    fn game_running(&mut self) -> Result<bool, GameMonitorError> {
        self.refresh();
        Ok(self.system.processes().values().any(process_matches))
    }
}

/// One-shot scan returning the names of running Path of Exile processes.
pub fn list_matching_processes() -> Result<Vec<String>, GameMonitorError> {
    Ok(SysinfoSource::new()?.matching_names())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    #[test]
    fn edge_detector_matches_poe2_monitor() {
        let mut d = EdgeDetector::new();
        assert_eq!(d.observe(false), None);
        assert_eq!(d.observe(true), Some(GameEvent::Started));
        assert_eq!(d.observe(true), Some(GameEvent::StillRunning));
        assert_eq!(d.observe(true), Some(GameEvent::StillRunning));
        assert_eq!(d.observe(false), Some(GameEvent::Stopped));
        assert_eq!(d.observe(false), None);
        // stop() resets: a game still running afterwards is reported as Started again.
        assert_eq!(d.observe(true), Some(GameEvent::Started));
        d.reset();
        assert!(!d.is_running());
        assert_eq!(d.observe(true), Some(GameEvent::Started));
    }

    #[test]
    fn thread_emits_started_still_running_stopped_once() {
        // Simulated process lists, one per tick; the script is consumed by the fake source.
        let script: Arc<Mutex<Vec<Vec<&'static str>>>> = Arc::new(Mutex::new(vec![
            vec!["PathOfExileSteam.exe", "explorer.exe"],
            vec!["explorer.exe", "Path of Exile 2"],
            vec!["explorer.exe"],
            vec!["explorer.exe"],
            vec!["explorer.exe"],
        ]));
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        let source = {
            let script = Arc::clone(&script);
            move || -> Result<bool, GameMonitorError> {
                let mut script = script.lock().expect("script lock");
                let list = if script.is_empty() {
                    Vec::new()
                } else {
                    script.remove(0)
                };
                Ok(list.iter().any(|name| is_path_of_exile_process(name)))
            }
        };
        let mut monitor =
            GameMonitor::start_with_source(Duration::from_millis(20), source, move |event| {
                sink.lock().expect("events lock").push(event)
            })
            .expect("monitor starts");

        let deadline = Instant::now() + Duration::from_secs(5);
        while script.lock().expect("script lock").len() > 1 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        thread::sleep(Duration::from_millis(100));
        monitor.stop();

        let events = events.lock().expect("events lock").clone();
        assert_eq!(
            events,
            vec![
                GameEvent::Started,
                GameEvent::StillRunning,
                GameEvent::Stopped
            ]
        );
    }

    #[test]
    fn query_errors_are_logged_and_keep_state() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        let mut tick = 0u32;
        let source = move || -> Result<bool, GameMonitorError> {
            tick += 1;
            match tick {
                1 => Ok(true),
                2 => Err(GameMonitorError::Query("simulated".into())),
                _ => Ok(true),
            }
        };
        let mut monitor =
            GameMonitor::start_with_source(Duration::from_millis(10), source, move |event| {
                sink.lock().expect("events lock").push(event)
            })
            .expect("monitor starts");
        let deadline = Instant::now() + Duration::from_secs(5);
        while events.lock().expect("events lock").len() < 2 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        monitor.stop();
        let events = events.lock().expect("events lock").clone();
        // Started on tick 1, tick 2 failed (no event, state kept), tick 3 is StillRunning, not Started.
        assert_eq!(&events[..2], &[GameEvent::Started, GameEvent::StillRunning]);
    }

    #[test]
    fn stop_is_idempotent_and_drop_is_safe() {
        let mut monitor =
            GameMonitor::start_with_source(Duration::from_millis(10), || Ok(false), |_| {})
                .expect("monitor starts");
        monitor.stop();
        monitor.stop();
        drop(monitor);
    }

    #[test]
    fn sysinfo_lists_processes_on_this_machine() {
        let matches = list_matching_processes().expect("sysinfo works on this box");
        eprintln!("matching processes: {matches:?}");
        // No Path of Exile here, but the scan itself must succeed and see processes.
        let mut source = SysinfoSource::new().expect("sysinfo");
        source.refresh();
        assert!(
            !source.system.processes().is_empty(),
            "sysinfo saw zero processes"
        );
        assert!(
            source
                .system
                .processes()
                .values()
                .all(|p| !p.name().is_empty()),
            "every process should have a name"
        );
    }
}
