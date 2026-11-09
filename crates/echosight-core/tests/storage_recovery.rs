use echosight_core::storage::{DataPaths, FileKind, Stage, Storage, StorageHooks};
use serde_json::{Value, json};
use std::{
    fs, io,
    path::Path,
    sync::{Arc, Mutex},
};

fn fixture() -> Value {
    serde_json::from_str(include_str!("golden/storage/storage.json")).unwrap()
}
fn kind(name: &str) -> FileKind {
    match name {
        "tasks" => FileKind::Tasks,
        "templates" => FileKind::Templates,
        "settings" => FileKind::Settings,
        _ => panic!(),
    }
}
#[derive(Default)]
struct Probe {
    fail: Mutex<Vec<Stage>>,
    events: Mutex<Vec<Stage>>,
}
impl StorageHooks for Probe {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        "2026-07-24T12:34:56.789Z".parse().unwrap()
    }
    fn before(&self, stage: Stage, _path: &Path) -> io::Result<()> {
        self.events.lock().unwrap().push(stage);
        let mut failures = self.fail.lock().unwrap();
        if failures.first() == Some(&stage) {
            failures.remove(0);
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("injected {stage:?}"),
            ));
        }
        Ok(())
    }
}

#[test]
fn explicit_paths_are_resolved_without_discovery_or_directory_creation() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("uncreated/data");
    let paths = DataPaths::new(&data).unwrap();
    assert_eq!(paths.file(FileKind::Tasks), data.join("current_tasks.json"));
    assert_eq!(paths.themes(), data.join("themes"));
    assert!(!data.exists());
}

#[test]
fn missing_files_return_ts_fallbacks_without_creating_anything() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("nonexistent");
    let store = Storage::new(DataPaths::new(&data).unwrap());
    for case in fixture()["missing"].as_array().unwrap() {
        assert_eq!(
            store.load(kind(case["kind"].as_str().unwrap())).unwrap(),
            case["value"]
        );
    }
    assert!(!data.exists());
}

#[test]
fn corrupt_bytes_are_preserved_before_ts_fallback_and_only_tasks_expose_backup() {
    for case in fixture()["recovery"].as_array().unwrap() {
        let root = tempfile::tempdir().unwrap();
        let paths = DataPaths::new(root.path()).unwrap();
        let kind = kind(case["kind"].as_str().unwrap());
        let path = paths.file(kind);
        let bytes: Vec<u8> = serde_json::from_value(case["input"].clone()).unwrap();
        fs::write(&path, &bytes).unwrap();
        let probe = Arc::new(Probe::default());
        let store = Storage::with_hooks(paths, probe.clone());
        let mut loaded = store.load(kind).unwrap();
        if kind == FileKind::Tasks {
            let backup = loaded["corruptBackupPath"].as_str().unwrap();
            loaded["corruptBackupPath"] =
                json!(Path::new(backup).file_name().unwrap().to_str().unwrap());
        }
        assert_eq!(loaded, case["value"]);
        assert_eq!(fs::read_to_string(&path).unwrap(), case["live"]);
        let backup = path.with_file_name(format!(
            "{}.corrupt-2026-07-24T12-34-56-789Z",
            path.file_name().unwrap().to_str().unwrap()
        ));
        assert_eq!(fs::read(backup).unwrap(), bytes);
        let events = probe.events.lock().unwrap();
        assert!(
            events.iter().position(|e| *e == Stage::Backup).unwrap()
                < events.iter().position(|e| *e == Stage::Replace).unwrap()
        );
    }
}

#[test]
fn load_failures_block_each_domain_until_successful_reload() {
    for kind in [FileKind::Tasks, FileKind::Templates, FileKind::Settings] {
        for stage in [
            Stage::Read,
            Stage::Backup,
            Stage::CreateTemp,
            Stage::Sync,
            Stage::Replace,
        ] {
            let root = tempfile::tempdir().unwrap();
            let paths = DataPaths::new(root.path()).unwrap();
            let path = paths.file(kind);
            let bytes = b"{broken";
            fs::write(&path, bytes).unwrap();
            let probe = Arc::new(Probe::default());
            *probe.fail.lock().unwrap() = vec![stage];
            let store = Storage::with_hooks(paths, probe.clone());
            assert!(
                store
                    .load(kind)
                    .unwrap_err()
                    .to_string()
                    .contains(&format!("{stage:?}"))
            );
            let before = probe.events.lock().unwrap().len();
            let blocked = store
                .save(kind, &json!({"replacement":1}))
                .unwrap_err()
                .to_string();
            assert!(blocked.starts_with("Save blocked after load failure: "));
            assert!(blocked.contains(&format!("{stage:?}")));
            assert_eq!(
                probe.events.lock().unwrap().len(),
                before,
                "blocked saves must not touch disk"
            );
            assert_eq!(fs::read(&path).unwrap(), bytes);
            if !matches!(stage, Stage::Read | Stage::Backup) {
                let backup = root.path().join(format!(
                    "{}.corrupt-2026-07-24T12-34-56-789Z",
                    path.file_name().unwrap().to_str().unwrap()
                ));
                assert_eq!(fs::read(backup).unwrap(), bytes);
            } else {
                assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
            }
            fs::write(&path, b"{\"recovered\":true}").unwrap();
            // External repair does not alone clear the write block.
            assert!(store.save(kind, &json!(false)).is_err());
            assert_eq!(store.load(kind).unwrap(), json!({"recovered":true}));
            store.save(kind, &json!({"replacement":2})).unwrap();
            assert_eq!(store.load(kind).unwrap(), json!({"replacement":2}));
        }
    }
}

#[test]
fn unsupported_valid_js_values_block_without_quarantine_or_replacement() {
    for input in ["undefined", "NaN", "Infinity", "1e400", "\"\\ud800\""] {
        let root = tempfile::tempdir().unwrap();
        let paths = DataPaths::new(root.path()).unwrap();
        let path = paths.file(FileKind::Tasks);
        fs::write(&path, input).unwrap();
        let store = Storage::new(paths);
        assert!(store.load(FileKind::Tasks).is_err());
        assert!(store.save(FileKind::Tasks, &json!([])).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), input);
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }
}

#[test]
fn successful_missing_reload_clears_block_and_other_domains_remain_writable() {
    let root = tempfile::tempdir().unwrap();
    let paths = DataPaths::new(root.path()).unwrap();
    let path = paths.file(FileKind::Tasks);
    fs::write(&path, b"{").unwrap();
    let probe = Arc::new(Probe::default());
    *probe.fail.lock().unwrap() = vec![Stage::Backup];
    let store = Storage::with_hooks(paths, probe);
    assert!(store.load(FileKind::Tasks).is_err());
    store.save(FileKind::Settings, &json!({"ok":true})).unwrap();
    fs::remove_file(&path).unwrap();
    store.load(FileKind::Tasks).unwrap();
    assert!(!path.exists());
    store.save(FileKind::Tasks, &json!([])).unwrap();
}

#[test]
fn save_failures_alone_do_not_create_a_load_block() {
    let root = tempfile::tempdir().unwrap();
    let probe = Arc::new(Probe::default());
    *probe.fail.lock().unwrap() = vec![Stage::Replace];
    let store = Storage::with_hooks(DataPaths::new(root.path()).unwrap(), probe);
    assert!(store.save(FileKind::Tasks, &json!([])).is_err());
    store.save(FileKind::Tasks, &json!([])).unwrap();
}

#[test]
fn successful_values_are_not_schema_normalized() {
    let root = tempfile::tempdir().unwrap();
    let paths = DataPaths::new(root.path()).unwrap();
    fs::write(paths.file(FileKind::Tasks), " TRUE ").unwrap();
    assert_eq!(
        Storage::new(paths).load(FileKind::Tasks).unwrap(),
        json!(true)
    );
}
