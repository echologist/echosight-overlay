use echosight_core::storage::{AtomicWriter, Stage, StorageHooks};
use serde_json::{Value, json};
use std::{
    fs, io,
    path::Path,
    sync::{Arc, Mutex, mpsc},
    thread,
};

#[derive(Default)]
struct Probe {
    fail: Mutex<Vec<Stage>>,
    events: Mutex<Vec<Stage>>,
}
impl StorageHooks for Probe {
    fn before(&self, stage: Stage, _path: &Path) -> io::Result<()> {
        self.events.lock().unwrap().push(stage);
        let mut failures = self.fail.lock().unwrap();
        if failures.first() == Some(&stage) {
            failures.remove(0);
            return Err(io::Error::other(format!("injected {stage:?}")));
        }
        Ok(())
    }
}

#[test]
fn creates_and_overwrites_existing_destination_with_ts_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/tasks.json");
    let writer = AtomicWriter::default();
    let fixture: Value = serde_json::from_str(include_str!("golden/storage/storage.json")).unwrap();
    for case in fixture["writes"].as_array().unwrap() {
        writer.write(&path, &case["value"]).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), case["bytes"]);
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }
}

#[test]
fn write_flush_close_and_replace_failures_preserve_old_bytes_and_cleanup() {
    for stage in [
        Stage::CreateDir,
        Stage::CreateTemp,
        Stage::Write,
        Stage::Sync,
        Stage::Close,
        Stage::Replace,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.json");
        fs::write(&path, b"old bytes").unwrap();
        let probe = Arc::new(Probe::default());
        *probe.fail.lock().unwrap() = vec![stage];
        let writer = AtomicWriter::with_hooks(probe);
        assert!(
            writer
                .write(&path, &json!({"caller":1}))
                .unwrap_err()
                .to_string()
                .contains(&format!("{stage:?}"))
        );
        assert_eq!(fs::read(&path).unwrap(), b"old bytes");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
        writer.write(&path, &json!({"caller":2})).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&fs::read(&path).unwrap()).unwrap(),
            json!({"caller":2})
        );
    }
}

#[test]
fn cleanup_error_does_not_replace_decisive_failure() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tasks.json");
    fs::write(&path, b"old").unwrap();
    let probe = Arc::new(Probe::default());
    *probe.fail.lock().unwrap() = vec![Stage::Replace, Stage::Cleanup];
    let error = AtomicWriter::with_hooks(probe)
        .write(&path, &json!(1))
        .unwrap_err();
    assert!(error.to_string().contains("Replace"));
    assert_eq!(fs::read(&path).unwrap(), b"old");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}

struct Gate {
    target: std::path::PathBuf,
    queued: mpsc::Sender<()>,
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
    first: std::sync::atomic::AtomicBool,
    fail: bool,
}
impl StorageHooks for Gate {
    fn enqueued(&self, path: &Path) {
        if path == self.target {
            self.queued.send(()).unwrap();
        }
    }
    fn before(&self, stage: Stage, path: &Path) -> io::Result<()> {
        if stage == Stage::Replace
            && path == self.target
            && self.first.swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            self.entered.send(()).unwrap();
            self.release
                .lock()
                .unwrap()
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
            if self.fail {
                return Err(io::Error::other("first replacement failed"));
            }
        }
        Ok(())
    }
}

#[test]
fn fifo_resolved_paths_across_writer_instances_continue_after_failure() {
    for fail in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.json");
        let alias = dir.path().join("unused/../tasks.json");
        let (queued_tx, queued_rx) = mpsc::channel();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let hooks = Arc::new(Gate {
            target: path.clone(),
            queued: queued_tx,
            entered: entered_tx,
            release: Mutex::new(release_rx),
            first: true.into(),
            fail,
        });
        let first = AtomicWriter::with_hooks(hooks.clone());
        let second = AtomicWriter::with_hooks(hooks);
        let path1 = path.clone();
        let a = thread::spawn(move || first.write(&path1, &json!({"caller":1})));
        queued_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        let b = thread::spawn(move || second.write(&alias, &json!({"caller":2})));
        queued_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap(); // second has a FIFO ticket while first is held
        // An unrelated path can complete while the first path is locked.
        AtomicWriter::default()
            .write(&dir.path().join("other.json"), &json!(true))
            .unwrap();
        release_tx.send(()).unwrap();
        assert_eq!(a.join().unwrap().is_err(), fail);
        b.join().unwrap().unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&fs::read(path).unwrap()).unwrap(),
            json!({"caller":2})
        );
    }
}

#[test]
fn sync_and_close_precede_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let probe = Arc::new(Probe::default());
    AtomicWriter::with_hooks(probe.clone())
        .write(&dir.path().join("tasks.json"), &json!([]))
        .unwrap();
    assert_eq!(
        *probe.events.lock().unwrap(),
        vec![
            Stage::CreateDir,
            Stage::CreateTemp,
            Stage::Write,
            Stage::Sync,
            Stage::Close,
            Stage::Replace
        ]
    );
}

#[cfg(windows)]
#[test]
fn windows_open_destination_without_delete_sharing_blocks_replace_without_deleting_it() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tasks.json");
    fs::write(&path, b"old").unwrap();
    // FILE_SHARE_READ | FILE_SHARE_WRITE deliberately omits FILE_SHARE_DELETE.
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0x1 | 0x2)
        .open(&path)
        .unwrap();
    assert!(AtomicWriter::default().write(&path, &json!("new")).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"old");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    drop(held);
    AtomicWriter::default().write(&path, &json!("new")).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"\"new\"");
}
