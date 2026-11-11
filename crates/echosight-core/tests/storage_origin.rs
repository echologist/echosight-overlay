use echosight_core::storage::{
    DataPaths, FileKind, LoadOrigin, LoadedValue, Stage, Storage, StorageHooks,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

const KINDS: [FileKind; 3] = [FileKind::Tasks, FileKind::Templates, FileKind::Settings];
const STAMP: &str = "2026-07-24T12-34-56-789Z";

#[derive(Default)]
struct Probe {
    fail: Mutex<Vec<Stage>>,
    events: Mutex<Vec<Stage>>,
}
impl Probe {
    fn failing(stage: Stage) -> Arc<Self> {
        let probe = Arc::new(Self::default());
        *probe.fail.lock().unwrap() = vec![stage];
        probe
    }
    fn events(&self) -> Vec<Stage> {
        self.events.lock().unwrap().clone()
    }
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

fn fallback(kind: FileKind) -> Value {
    match kind {
        FileKind::Tasks => json!({"tasks": [], "currentTemplate": null, "snapshots": []}),
        FileKind::Templates => json!([]),
        FileKind::Settings => Value::Null,
    }
}
fn fallback_disk(kind: FileKind) -> Vec<u8> {
    echosight_core::storage::json_format(&fallback(kind))
        .unwrap()
        .into_bytes()
}
fn backup_path(path: &Path) -> PathBuf {
    path.with_file_name(format!(
        "{}.corrupt-{STAMP}",
        path.file_name().unwrap().to_str().unwrap()
    ))
}
fn tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let rel = path.strip_prefix(base).unwrap().to_path_buf();
            if path.is_dir() {
                out.insert(rel, None);
                walk(base, &path, out);
            } else {
                out.insert(rel, Some(fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = BTreeMap::new();
    if root.exists() {
        walk(root, root, &mut out);
    }
    out
}
/// Normalizes the one machine-specific field (absolute backup path) for comparison.
fn normalize(mut value: Value) -> Value {
    if let Some(p) = value.get("corruptBackupPath").and_then(Value::as_str) {
        let name = Path::new(p)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        value["corruptBackupPath"] = json!(name);
    }
    value
}
fn setup(kind: FileKind, probe: Arc<Probe>) -> (tempfile::TempDir, PathBuf, Storage) {
    let root = tempfile::tempdir().unwrap();
    let paths = DataPaths::new(root.path()).unwrap();
    let path = paths.file(kind);
    (root, path, Storage::with_hooks(paths, probe))
}

#[test]
fn missing_nested_root_stays_absent_with_exact_fallback_and_single_read_probe() {
    for kind in KINDS {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("a/b/data");
        let probe = Arc::new(Probe::default());
        let store = Storage::with_hooks(DataPaths::new(&data).unwrap(), probe.clone());
        let loaded = store.load_with_origin(kind).unwrap();
        assert_eq!(
            loaded,
            LoadedValue {
                value: fallback(kind),
                origin: LoadOrigin::Missing
            }
        );
        assert!(!root.path().join("a").exists());
        assert!(tree(root.path()).is_empty());
        assert_eq!(probe.events(), vec![Stage::Read]);
        // Missing load does not block saves.
        store.save(kind, &json!({"x": 1})).unwrap();
    }
}

#[test]
fn explicit_null_is_existing_and_valid_reads_do_not_write() {
    for kind in KINDS {
        for (bytes, expected) in [
            (&b"null"[..], Value::Null),
            (b" {\"a\": [1, 2]} ", json!({"a": [1, 2]})),
            (b"[]", json!([])),
        ] {
            let probe = Arc::new(Probe::default());
            let (root, path, store) = setup(kind, probe.clone());
            fs::write(&path, bytes).unwrap();
            let before = tree(root.path());
            let loaded = store.load_with_origin(kind).unwrap();
            assert_eq!(loaded.origin, LoadOrigin::Existing);
            assert_eq!(loaded.value, expected);
            assert_eq!(tree(root.path()), before);
            assert_eq!(probe.events(), vec![Stage::Read]);
        }
    }
    // Null is distinct from missing for settings (same value, different origin).
    let (_root, path, store) = setup(FileKind::Settings, Arc::default());
    assert_eq!(
        store.load_with_origin(FileKind::Settings).unwrap().origin,
        LoadOrigin::Missing
    );
    fs::write(&path, "null").unwrap();
    assert_eq!(
        store.load_with_origin(FileKind::Settings).unwrap().origin,
        LoadOrigin::Existing
    );
}

#[test]
fn corrupt_bytes_are_backed_up_exactly_and_fallback_written_before_recovered() {
    for kind in KINDS {
        let corrupt: &[u8] = b"{broken \xff\xfe bytes";
        let probe = Arc::new(Probe::default());
        let (root, path, store) = setup(kind, probe.clone());
        fs::write(&path, corrupt).unwrap();
        let loaded = store.load_with_origin(kind).unwrap();
        assert_eq!(loaded.origin, LoadOrigin::Recovered);
        let backup = backup_path(&path);
        assert_eq!(fs::read(&backup).unwrap(), corrupt);
        let disk: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(disk, fallback(kind));
        assert!(disk.get("corruptBackupPath").is_none());
        assert_eq!(fs::read(&path).unwrap(), fallback_disk(kind));
        if kind == FileKind::Tasks {
            assert_eq!(
                loaded.value["corruptBackupPath"].as_str().unwrap(),
                backup.to_string_lossy()
            );
            let mut stripped = loaded.value.clone();
            stripped
                .as_object_mut()
                .unwrap()
                .remove("corruptBackupPath");
            assert_eq!(stripped, fallback(kind));
        } else {
            assert_eq!(loaded.value, fallback(kind));
        }
        assert_eq!(tree(root.path()).len(), 2);
        assert_eq!(
            probe.events(),
            vec![
                Stage::Read,
                Stage::Backup,
                Stage::CreateDir,
                Stage::CreateTemp,
                Stage::Write,
                Stage::Sync,
                Stage::Close,
                Stage::Replace
            ]
        );
        // Origin is not persisted: a second load reads the fallback as Existing.
        let again = store.load_with_origin(kind).unwrap();
        assert_eq!(again.origin, LoadOrigin::Existing);
        assert_eq!(again.value, fallback(kind));
    }
}

#[test]
fn faults_never_return_origin_and_block_clones_until_later_success() {
    let stages = [
        Stage::Read,
        Stage::Backup,
        Stage::CreateDir,
        Stage::CreateTemp,
        Stage::Write,
        Stage::Sync,
        Stage::Close,
        Stage::Replace,
    ];
    for kind in KINDS {
        for stage in stages {
            let corrupt = b"{broken";
            let probe = Probe::failing(stage);
            let (root, path, store) = setup(kind, probe.clone());
            let clone = store.clone();
            fs::write(&path, corrupt).unwrap();
            let error = store.load_with_origin(kind).unwrap_err().to_string();
            assert!(error.contains(&format!("injected {stage:?}")), "{error}");
            let events = probe.events();
            // Cleanup runs only once the temp file has been created.
            assert_eq!(
                events.last() == Some(&Stage::Cleanup),
                matches!(
                    stage,
                    Stage::Write | Stage::Sync | Stage::Close | Stage::Replace
                )
            );
            // Live bytes are never lost; no stray temp files remain.
            assert_eq!(fs::read(&path).unwrap(), corrupt);
            let names: Vec<String> = tree(root.path())
                .keys()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            assert!(names.iter().all(|n| !n.ends_with(".tmp")), "{names:?}");
            let has_backup = !matches!(stage, Stage::Read | Stage::Backup);
            assert_eq!(backup_path(&path).exists(), has_backup);
            // Blocked through the clone, without touching disk.
            let before = probe.events().len();
            let blocked = clone.save(kind, &json!({"r": 1})).unwrap_err().to_string();
            assert!(blocked.starts_with("Save blocked after load failure: "));
            assert!(blocked.contains(&format!("{stage:?}")));
            assert_eq!(probe.events().len(), before);
            assert_eq!(fs::read(&path).unwrap(), corrupt);
            // A later successful load clears the block.
            fs::write(&path, b"null").unwrap();
            let loaded = clone.load_with_origin(kind).unwrap();
            assert_eq!(loaded.origin, LoadOrigin::Existing);
            store.save(kind, &json!({"r": 2})).unwrap();
        }
    }
}

#[test]
fn read_fault_on_existing_valid_file_returns_no_origin_and_blocks() {
    for kind in KINDS {
        let probe = Probe::failing(Stage::Read);
        let (root, path, store) = setup(kind, probe.clone());
        fs::write(&path, "null").unwrap();
        assert!(store.load_with_origin(kind).is_err());
        assert_eq!(probe.events(), vec![Stage::Read]);
        assert!(store.clone().save(kind, &json!(1)).is_err());
        assert_eq!(tree(root.path()).len(), 1);
        assert_eq!(
            store.load_with_origin(kind).unwrap().origin,
            LoadOrigin::Existing
        );
        store.save(kind, &json!(1)).unwrap();
    }
}

#[test]
fn unsupported_values_fail_without_quarantine_and_later_load_unblocks() {
    for kind in KINDS {
        for input in ["undefined", "NaN", "Infinity", "1e400", "\"\\ud800\""] {
            let probe = Arc::new(Probe::default());
            let (root, path, store) = setup(kind, probe.clone());
            fs::write(&path, input).unwrap();
            let before = tree(root.path());
            assert!(store.load_with_origin(kind).is_err(), "{input}");
            assert_eq!(probe.events(), vec![Stage::Read]);
            assert_eq!(tree(root.path()), before);
            assert!(store.clone().save(kind, &json!([])).is_err());
            assert_eq!(tree(root.path()), before);
            assert_eq!(probe.events(), vec![Stage::Read]);
            fs::write(&path, "[]").unwrap();
            let loaded = store.load_with_origin(kind).unwrap();
            assert_eq!(loaded.origin, LoadOrigin::Existing);
            store.save(kind, &json!([1])).unwrap();
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Scenario {
    Missing,
    Null,
    Valid,
    Corrupt,
    Unsupported,
    Fault(Stage),
}

type Tree = BTreeMap<PathBuf, Option<Vec<u8>>>;
type RunOutcome = (String, Vec<Stage>, Tree, String);

fn run(kind: FileKind, scenario: Scenario, origin_api: bool) -> RunOutcome {
    let probe = match scenario {
        Scenario::Fault(stage) => Probe::failing(stage),
        _ => Arc::new(Probe::default()),
    };
    let (root, path, store) = setup(kind, probe.clone());
    match scenario {
        Scenario::Missing => {}
        Scenario::Null => fs::write(&path, "null").unwrap(),
        Scenario::Valid => fs::write(&path, "{\"k\":[true]}").unwrap(),
        Scenario::Corrupt | Scenario::Fault(_) => fs::write(&path, "{oops").unwrap(),
        Scenario::Unsupported => fs::write(&path, "NaN").unwrap(),
    }
    let result = if origin_api {
        store.load_with_origin(kind).map(|l| l.value)
    } else {
        store.load(kind)
    };
    let outcome = match result {
        Ok(value) => format!("ok {}", normalize(value)),
        Err(error) => format!("err {error}"),
    };
    let blocked = match store.save(kind, &json!({"after": 1})) {
        Ok(()) => "saved".to_owned(),
        Err(error) => error.to_string(),
    };
    (outcome, probe.events(), tree(root.path()), blocked)
}

#[test]
fn legacy_load_matches_origin_load_values_effects_and_block_state() {
    let mut scenarios = vec![
        Scenario::Missing,
        Scenario::Null,
        Scenario::Valid,
        Scenario::Corrupt,
        Scenario::Unsupported,
    ];
    scenarios.extend(
        [
            Stage::Read,
            Stage::Backup,
            Stage::CreateDir,
            Stage::CreateTemp,
            Stage::Write,
            Stage::Sync,
            Stage::Close,
            Stage::Replace,
        ]
        .map(Scenario::Fault),
    );
    for kind in KINDS {
        for scenario in &scenarios {
            let legacy = run(kind, *scenario, false);
            let with_origin = run(kind, *scenario, true);
            assert_eq!(legacy, with_origin, "{kind:?} {scenario:?}");
        }
    }
    // Legacy return shapes stay exact.
    let (_root, path, store) = setup(FileKind::Tasks, Arc::default());
    assert_eq!(
        store.load(FileKind::Tasks).unwrap(),
        fallback(FileKind::Tasks)
    );
    fs::write(&path, "{").unwrap();
    let recovered = store.load(FileKind::Tasks).unwrap();
    assert_eq!(
        recovered["corruptBackupPath"].as_str().unwrap(),
        backup_path(&path).to_string_lossy()
    );
}
