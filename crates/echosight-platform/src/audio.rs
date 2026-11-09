//! Sound playback for theme sound events (`taskCompleted`, `backgroundActivated`,
//! `undo`, `redo`), ported from `src/renderer/features/themes/themeSounds.ts`.
//!
//! Design:
//! - All device work happens on one dedicated worker thread owned by
//!   [`SoundPlayer`]. The rodio/cpal output stream ([`rodio::MixerDeviceSink`])
//!   is opened lazily on the first [`SoundPlayer::play`] call, never at startup,
//!   so an app without an audio device (or with sounds disabled) pays nothing.
//! - Decoded PCM is cached per `(theme id, event)` so each sound is decoded once
//!   per theme; the cache is cleared with [`SoundPlayer::clear_cache`] when a
//!   theme is applied (mirrors `audioCache` in the TypeScript code).
//! - Every failure (no device, bad file, closed worker) is logged at `warn`
//!   level and never panics: sounds are best effort, exactly like the
//!   `catch` + `console.warn` in the renderer.
//!
//! Supported containers/codecs come from these rodio 0.23 cargo features
//! (names verified against rodio 0.23.0's `Cargo.toml`): `playback` (cpal
//! output), `wav` (= `symphonia-wav` + `symphonia-pcm`), `vorbis`
//! (= `symphonia-ogg` + `symphonia-vorbis`), `mp3` (= `symphonia-mp3`) and
//! `mp4` (= `symphonia-isomp4` + `symphonia-aac`, which covers `.m4a`).

use std::collections::HashMap;
use std::fmt;
use std::io::Cursor;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use rodio::buffer::SamplesBuffer;
use rodio::{ChannelCount, Decoder, DeviceSinkBuilder, MixerDeviceSink, SampleRate, Source};

/// Volume used when `settings.sounds.volume` is missing or not a finite number
/// (`normalizeUnitVolume(settings.volume / 100, 0.6)`).
pub const DEFAULT_SETTINGS_VOLUME: f32 = 0.6;
/// Volume used when `theme.sounds.volume` is missing (`normalizeUnitVolume(theme.sounds?.volume, 1)`).
pub const DEFAULT_THEME_VOLUME: f32 = 1.0;
/// Volume used when a sound event definition has no `volume` (string form, or missing key).
pub const DEFAULT_EVENT_VOLUME: f32 = 1.0;

/// How long the worker waits before trying to open the output device again
/// after a failed attempt (avoids re-enumerating devices on every click).
const DEVICE_RETRY_BACKOFF: Duration = Duration::from_secs(10);

/// The four theme sound events (`ThemeSoundEvent` in `src/shared/types.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundEvent {
    TaskCompleted,
    BackgroundActivated,
    Undo,
    Redo,
}

impl SoundEvent {
    /// The JSON key used in `theme.sounds.events`.
    pub fn key(self) -> &'static str {
        match self {
            SoundEvent::TaskCompleted => "taskCompleted",
            SoundEvent::BackgroundActivated => "backgroundActivated",
            SoundEvent::Undo => "undo",
            SoundEvent::Redo => "redo",
        }
    }
}

/// Cache key: one decoded buffer per theme and event.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SoundKey {
    pub theme_id: String,
    pub event: SoundEvent,
}

impl SoundKey {
    pub fn new(theme_id: impl Into<String>, event: SoundEvent) -> Self {
        Self {
            theme_id: theme_id.into(),
            event,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("failed to decode sound: {0}")]
    Decode(#[from] rodio::decoder::DecoderError),
    #[error("decoded sound contains no samples")]
    Empty,
    #[error("failed to open the default audio output: {0}")]
    Device(#[from] rodio::DeviceSinkError),
    #[error("audio worker thread is not running")]
    WorkerGone,
}

/// Fully decoded PCM, interleaved `f32` samples.
#[derive(Clone)]
pub struct DecodedSound {
    channels: ChannelCount,
    sample_rate: SampleRate,
    samples: Arc<[f32]>,
}

impl DecodedSound {
    pub fn channels(&self) -> u16 {
        self.channels.get()
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate.get()
    }

    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// Duration of the clip.
    pub fn duration(&self) -> Duration {
        let frames = self.samples.len() as u64 / u64::from(self.channels.get());
        Duration::from_secs_f64(frames as f64 / f64::from(self.sample_rate.get()))
    }

    fn source(&self) -> SamplesBuffer {
        SamplesBuffer::new(self.channels, self.sample_rate, self.samples.to_vec())
    }
}

impl fmt::Debug for DecodedSound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DecodedSound")
            .field("channels", &self.channels)
            .field("sample_rate", &self.sample_rate)
            .field("samples", &self.samples.len())
            .finish()
    }
}

/// Decodes a whole sound file held in memory. Format is sniffed from the
/// bytes (wav / ogg-vorbis / mp3 / mp4-aac), the extension is not needed.
pub fn decode(bytes: Arc<[u8]>) -> Result<DecodedSound, AudioError> {
    let decoder = Decoder::new(Cursor::new(bytes))?;
    let channels = decoder.channels();
    let sample_rate = decoder.sample_rate();
    let samples: Vec<f32> = decoder.collect();
    if samples.is_empty() {
        return Err(AudioError::Empty);
    }
    Ok(DecodedSound {
        channels,
        sample_rate,
        samples: samples.into(),
    })
}

/// Mirrors `normalizeUnitVolume(value, fallback)`: finite numbers are clamped
/// to `0..=1`, anything else (missing, NaN, infinite) yields `fallback`.
pub fn normalize_unit_volume(value: Option<f64>, fallback: f32) -> f32 {
    match value {
        Some(v) if v.is_finite() => v.clamp(0.0, 1.0) as f32,
        _ => fallback,
    }
}

/// `calculatePlaybackVolume` from `themeSounds.ts`:
/// `clamp(settings.volume / 100, fallback 0.6) * clamp(theme.sounds.volume, fallback 1) * event volume`.
///
/// `settings_volume_percent` is the raw `settings.sounds.volume` (0..100),
/// `theme_volume` is `theme.sounds.volume`, `event_volume` is the per-event
/// `volume` (already normalised to `1` when the event is given as a plain
/// string; `None` here means the same).
pub fn effective_volume(
    settings_volume_percent: Option<f64>,
    theme_volume: Option<f64>,
    event_volume: Option<f64>,
) -> f32 {
    normalize_unit_volume(
        settings_volume_percent.map(|v| v / 100.0),
        DEFAULT_SETTINGS_VOLUME,
    ) * normalize_unit_volume(theme_volume, DEFAULT_THEME_VOLUME)
        * normalize_unit_volume(event_volume, DEFAULT_EVENT_VOLUME)
}

/// The sound gate: `settings.sounds.enabled && theme.sounds.enabled !== false`.
pub fn sounds_allowed(settings_enabled: bool, theme_enabled: Option<bool>) -> bool {
    settings_enabled && theme_enabled != Some(false)
}

enum Command {
    Play {
        key: Option<SoundKey>,
        bytes: Arc<[u8]>,
        volume: f32,
    },
    ClearCache,
    /// Test/diagnostic hook: reports whether an output device could be opened.
    Probe(Sender<bool>),
}

/// Thread-safe, lazily initialised sound player. Cheap to clone-by-`Arc`; the
/// worker thread (and the output stream) are dropped with the last handle.
pub struct SoundPlayer {
    tx: Mutex<Option<Sender<Command>>>,
}

impl Default for SoundPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundPlayer {
    /// Creates the player. No thread and no audio device is touched until the
    /// first [`SoundPlayer::play`].
    pub fn new() -> Self {
        Self {
            tx: Mutex::new(None),
        }
    }

    /// Plays `bytes` once at `volume` (0..=1), decoding them every time.
    /// Failures are logged, never returned.
    pub fn play(&self, bytes: Arc<[u8]>, volume: f32) {
        self.send(Command::Play {
            key: None,
            bytes,
            volume,
        });
    }

    /// Plays a theme event sound, decoding `bytes` only the first time `key`
    /// is seen (until [`SoundPlayer::clear_cache`]).
    pub fn play_event(&self, key: SoundKey, bytes: Arc<[u8]>, volume: f32) {
        self.send(Command::Play {
            key: Some(key),
            bytes,
            volume,
        });
    }

    /// Drops every cached decoded sound (call when the theme changes).
    pub fn clear_cache(&self) {
        self.send(Command::ClearCache);
    }

    /// Opens (or reuses) the output device and reports whether that worked.
    /// Blocks until the worker answers. Intended for diagnostics and spikes.
    pub fn probe_output(&self) -> bool {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send(Command::Probe(reply_tx));
        reply_rx.recv().unwrap_or(false)
    }

    fn send(&self, command: Command) {
        let Ok(mut guard) = self.tx.lock() else {
            log::warn!("sound player lock poisoned; dropping sound command");
            return;
        };
        if guard.is_none() {
            *guard = spawn_worker();
        }
        let Some(tx) = guard.as_ref() else {
            return;
        };
        if tx.send(command).is_err() {
            log::warn!("{}", AudioError::WorkerGone);
            *guard = None;
        }
    }
}

fn spawn_worker() -> Option<Sender<Command>> {
    let (tx, rx) = mpsc::channel::<Command>();
    let spawned = thread::Builder::new()
        .name("echosight-audio".into())
        .spawn(move || {
            let mut worker = Worker::default();
            while let Ok(command) = rx.recv() {
                worker.handle(command);
            }
        });
    match spawned {
        Ok(_) => Some(tx),
        Err(err) => {
            log::warn!("failed to spawn audio worker thread: {err}");
            None
        }
    }
}

#[derive(Default)]
struct Worker {
    sink: Option<MixerDeviceSink>,
    last_failed_open: Option<Instant>,
    cache: HashMap<SoundKey, DecodedSound>,
}

impl Worker {
    fn handle(&mut self, command: Command) {
        match command {
            Command::Play { key, bytes, volume } => {
                if let Err(err) = self.play(key, bytes, volume) {
                    log::warn!("sound playback failed: {err}");
                }
            }
            Command::ClearCache => self.cache.clear(),
            Command::Probe(reply) => {
                let ok = self.ensure_sink().is_ok();
                let _ = reply.send(ok);
            }
        }
    }

    fn play(
        &mut self,
        key: Option<SoundKey>,
        bytes: Arc<[u8]>,
        volume: f32,
    ) -> Result<(), AudioError> {
        let sound = match key {
            Some(key) => {
                if let Some(cached) = self.cache.get(&key) {
                    cached.clone()
                } else {
                    let decoded = decode(bytes)?;
                    self.cache.insert(key, decoded.clone());
                    decoded
                }
            }
            None => decode(bytes)?,
        };
        let sink = self.ensure_sink()?;
        sink.mixer()
            .add(sound.source().amplify(volume.clamp(0.0, 1.0)));
        Ok(())
    }

    fn ensure_sink(&mut self) -> Result<&MixerDeviceSink, AudioError> {
        if self.sink.is_none() {
            if let Some(failed_at) = self.last_failed_open
                && failed_at.elapsed() < DEVICE_RETRY_BACKOFF
            {
                return Err(AudioError::Device(rodio::DeviceSinkError::NoDevice));
            }
            match DeviceSinkBuilder::open_default_sink() {
                Ok(mut sink) => {
                    sink.log_on_drop(false);
                    log::info!("audio output opened: {:?}", sink.config());
                    self.sink = Some(sink);
                    self.last_failed_open = None;
                }
                Err(err) => {
                    self.last_failed_open = Some(Instant::now());
                    return Err(AudioError::Device(err));
                }
            }
        }
        self.sink.as_ref().ok_or(AudioError::WorkerGone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Arc<[u8]> {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/");
        std::fs::read(format!("{path}{name}"))
            .unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
            .into()
    }

    fn assert_tone(name: &str) -> DecodedSound {
        let decoded = decode(fixture(name)).unwrap_or_else(|e| panic!("decode {name}: {e}"));
        assert!(!decoded.samples().is_empty(), "{name}: no samples");
        assert_eq!(decoded.channels(), 1, "{name}: channels");
        assert_eq!(decoded.sample_rate(), 22_050, "{name}: sample rate");
        let dur = decoded.duration();
        assert!(
            (0.10..0.30).contains(&dur.as_secs_f64()),
            "{name}: duration {dur:?} not ~0.15 s"
        );
        let peak = decoded.samples().iter().fold(0f32, |m, s| m.max(s.abs()));
        assert!(
            peak > 0.1,
            "{name}: peak {peak} too quiet, decode is silent"
        );
        decoded
    }

    #[test]
    fn decodes_wav_fixture() {
        assert_tone("tone.wav");
    }

    #[test]
    fn decodes_ogg_vorbis_fixture() {
        assert_tone("tone.ogg");
    }

    #[test]
    fn decodes_mp3_fixture() {
        assert_tone("tone.mp3");
    }

    #[test]
    fn decodes_m4a_aac_fixture() {
        assert_tone("tone.m4a");
    }

    #[test]
    fn decodes_bundled_winamp_complete_wav() {
        let bytes: Arc<[u8]> = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/themes/winamp-classic/complete.wav"
        ))
        .expect("bundled complete.wav")
        .into();
        let decoded = decode(bytes).expect("decode complete.wav");
        assert!(!decoded.samples().is_empty());
        assert!(decoded.channels() >= 1);
        assert!(decoded.sample_rate() >= 8_000);
    }

    #[test]
    fn decode_rejects_garbage() {
        let bytes: Arc<[u8]> = b"definitely not audio".to_vec().into();
        assert!(matches!(decode(bytes), Err(AudioError::Decode(_))));
    }

    #[test]
    fn effective_volume_matches_theme_sounds_ts() {
        let close = |a: f32, b: f32| (a - b).abs() < 1e-6;
        assert!(close(effective_volume(None, None, None), 0.6));
        assert!(close(effective_volume(Some(60.0), None, None), 0.6));
        assert!(close(
            effective_volume(Some(100.0), Some(0.8), Some(0.5)),
            0.4
        ));
        assert!(close(effective_volume(Some(500.0), None, None), 1.0));
        assert!(close(effective_volume(Some(-5.0), None, None), 0.0));
        // Non-finite values fall back, they are not clamped.
        assert!(close(effective_volume(Some(f64::NAN), None, None), 0.6));
        assert!(close(
            effective_volume(Some(100.0), Some(f64::INFINITY), None),
            1.0
        ));
        // Event volume above 1 is clamped like the TS `normalizeUnitVolume(def.volume, 1)`.
        assert!(close(effective_volume(Some(100.0), None, Some(4.0)), 1.0));
    }

    #[test]
    fn sound_gate_matches_can_play_sounds() {
        assert!(sounds_allowed(true, None));
        assert!(sounds_allowed(true, Some(true)));
        assert!(!sounds_allowed(true, Some(false)));
        assert!(!sounds_allowed(false, None));
    }

    #[test]
    fn play_without_device_logs_and_does_not_panic() {
        // On the CI/Linux box there may or may not be an output device; either
        // way `play` must return normally and the worker must stay alive.
        let player = SoundPlayer::new();
        player.play(fixture("tone.wav"), 0.5);
        player.play_event(
            SoundKey::new("winamp-classic", SoundEvent::TaskCompleted),
            fixture("tone.ogg"),
            0.5,
        );
        player.play_event(
            SoundKey::new("winamp-classic", SoundEvent::TaskCompleted),
            fixture("tone.ogg"),
            0.5,
        );
        player.clear_cache();
        let has_device = player.probe_output();
        eprintln!("audio output device available: {has_device}");
        // Worker still answers after the earlier (possibly failing) plays.
        assert_eq!(player.probe_output(), has_device);
    }

    #[test]
    fn sound_event_keys_match_theme_json() {
        assert_eq!(SoundEvent::TaskCompleted.key(), "taskCompleted");
        assert_eq!(SoundEvent::BackgroundActivated.key(), "backgroundActivated");
        assert_eq!(SoundEvent::Undo.key(), "undo");
        assert_eq!(SoundEvent::Redo.key(), "redo");
    }
}
