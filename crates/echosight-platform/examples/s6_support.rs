//! Spike S6: audio + game monitor + single instance + tray, console only (no window).
//!
//! Run: `cargo run -p echosight-platform --example s6_support` (from the repo root).
//! Run a second copy while the first is alive: it prints `another instance is
//! already running`, the first prints `reveal requested`, and the second exits
//! with code 3.
//!
//! On Windows the tray is created on the main thread and a Win32 message loop
//! is pumped for the run's duration (tray-icon needs it; in the real app GPUI's
//! `GetMessageW` loop on the main thread does this job).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use echosight_platform::audio::SoundPlayer;
use echosight_platform::game_monitor::{self, GameEvent, GameMonitor};
use echosight_platform::single_instance::{ECHOSIGHT_APP_ID, SingleInstance, SingleInstanceError};
use echosight_platform::tray::{self, AppTray, Platform, TrayCallbacks, TrayIconRgba};

const RUN_FOR: Duration = Duration::from_secs(15);
const TICK: Duration = Duration::from_secs(3);
const EXIT_ALREADY_RUNNING: i32 = 3;

struct StderrLogger;

impl log::Log for StderrLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            eprintln!(
                "[{}] {}: {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

static LOGGER: StderrLogger = StderrLogger;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn main() {
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Info));
    let started = Instant::now();

    // 1. Single instance.
    let _guard = match SingleInstance::acquire(ECHOSIGHT_APP_ID, || println!("reveal requested")) {
        Ok(guard) => {
            println!(
                "single instance acquired ({})",
                SingleInstance::resource_name(ECHOSIGHT_APP_ID)
            );
            guard
        }
        Err(SingleInstanceError::AlreadyRunning) => {
            println!(
                "another instance is already running; signalled it, exiting with {EXIT_ALREADY_RUNNING}"
            );
            std::process::exit(EXIT_ALREADY_RUNNING);
        }
        Err(err) => {
            eprintln!("single instance failed: {err}");
            std::process::exit(1);
        }
    };

    // 2. Audio: fire-and-forget. The worker thread opens the output device on
    // first use and logs a warning if there is none, so this never blocks.
    // `S6_SKIP_AUDIO=1` leaves the device alone (useful to measure monitor CPU
    // on boxes where opening ALSA spins for seconds).
    let player = SoundPlayer::new();
    let wav_path = repo_root().join("data/themes/winamp-classic/complete.wav");
    if std::env::var_os("S6_SKIP_AUDIO").is_some() {
        println!("audio skipped (S6_SKIP_AUDIO set)");
    } else {
        match std::fs::read(&wav_path) {
            Ok(bytes) => {
                println!("requested playback of {} at volume 0.5", wav_path.display());
                player.play(Arc::from(bytes), 0.5);
            }
            Err(err) => println!("could not read {}: {err}", wav_path.display()),
        }
    }

    // 3. Tray (Windows only unless built with `tray-everywhere`).
    let exit_requested = Arc::new(AtomicBool::new(false));
    let _tray = if tray::should_create(Platform::current()) {
        let exit_flag = Arc::clone(&exit_requested);
        let callbacks = TrayCallbacks::new(
            || println!("tray: show"),
            || println!("tray: hide"),
            move || {
                println!("tray: exit");
                exit_flag.store(true, Ordering::SeqCst);
            },
        );
        let icon_path = repo_root().join("assets/tray-icon.ico");
        let tray = std::fs::read(&icon_path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| TrayIconRgba::from_ico_bytes(&bytes).map_err(|e| e.to_string()))
            .and_then(|icon| AppTray::new(icon, callbacks).map_err(|e| e.to_string()));
        match tray {
            Ok(tray) => {
                println!(
                    "tray created ({}); left-click = show, right-click = menu",
                    tray.backend()
                );
                Some(tray)
            }
            Err(err) => {
                println!("tray not created: {err}");
                None
            }
        }
    } else {
        println!(
            "tray skipped on {:?} (v2.1.1 parity: Windows only)",
            Platform::current()
        );
        None
    };

    // 4. Game monitor.
    let mut monitor = match GameMonitor::start(TICK, |event| {
        let label = match event {
            GameEvent::Started => "Started",
            GameEvent::Stopped => "Stopped",
            GameEvent::StillRunning => "StillRunning",
        };
        println!("game monitor: {label}");
    }) {
        Ok(monitor) => Some(monitor),
        Err(err) => {
            println!("game monitor failed to start: {err}");
            None
        }
    };

    // 5. Main loop: print matching processes every 3 s for 15 s, pump Win32 messages.
    let mut next_scan = Instant::now();
    while started.elapsed() < RUN_FOR && !exit_requested.load(Ordering::SeqCst) {
        if Instant::now() >= next_scan {
            match game_monitor::list_matching_processes() {
                Ok(names) if names.is_empty() => println!("matching processes: none"),
                Ok(names) => println!("matching processes: {}", names.join(", ")),
                Err(err) => println!("process scan failed: {err}"),
            }
            next_scan += TICK;
        }
        pump_messages();
        std::thread::sleep(Duration::from_millis(16));
    }

    if let Some(monitor) = monitor.as_mut() {
        monitor.stop();
    }
    let wall = started.elapsed().as_secs_f32();
    match process_cpu_seconds() {
        Some(cpu) => println!(
            "done after {wall:.1} s; process CPU time {cpu:.2} s ({:.1}% of one core)",
            cpu / wall * 100.0
        ),
        None => println!("done after {wall:.1} s"),
    }
}

/// User + system CPU seconds of this process (Linux only; `None` elsewhere).
#[cfg(target_os = "linux")]
fn process_cpu_seconds() -> Option<f32> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let after_comm = stat.rsplit(')').next()?;
    let fields: Vec<&str> = after_comm.split_whitespace().collect();
    let utime: f32 = fields.get(11)?.parse().ok()?;
    let stime: f32 = fields.get(12)?.parse().ok()?;
    Some((utime + stime) / 100.0)
}

#[cfg(not(target_os = "linux"))]
fn process_cpu_seconds() -> Option<f32> {
    None
}

#[cfg(windows)]
fn pump_messages() {
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
    };
    let mut msg = MSG::default();
    // SAFETY: `msg` is a valid, writable MSG for the duration of each call; no
    // window filter is applied. This mirrors a standard Win32 message pump.
    unsafe {
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

#[cfg(not(windows))]
fn pump_messages() {}
