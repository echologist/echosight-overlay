use echosight_core::storage::{DataPaths, FileKind, Stage, Storage, StorageHooks};
use serde_json::{Value, json};
use std::{fs, io, path::Path, sync::Arc};

#[test]
fn bootstrap_matches_real_ts_for_bundled_missing_corrupt_and_existing_files() {
    let fixture: Value = serde_json::from_str(include_str!("golden/storage/storage.json")).unwrap();
    for case in fixture["bootstrap"].as_array().unwrap() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("data");
        let defaults = root.path().join("defaults");
        fs::create_dir(&defaults).unwrap();
        for (name, bytes) in case["sources"].as_object().unwrap() {
            fs::write(defaults.join(name), bytes.as_str().unwrap()).unwrap();
        }
        if !case["existing"].as_object().unwrap().is_empty() {
            fs::create_dir(&data).unwrap();
            for (name, bytes) in case["existing"].as_object().unwrap() {
                fs::write(data.join(name), bytes.as_str().unwrap()).unwrap();
            }
        }
        let store = Storage::new(DataPaths::new(&data).unwrap());
        store.bootstrap(&defaults, &case["settings"]).unwrap();
        for (name, bytes) in case["files"].as_object().unwrap() {
            assert_eq!(
                fs::read_to_string(data.join(name)).unwrap(),
                bytes.as_str().unwrap(),
                "{} {name}",
                case["scenario"]
            );
        }
        // A later launch never resets any runtime file, including invalid bytes.
        store
            .bootstrap(&defaults, &json!({"theme":"must not overwrite"}))
            .unwrap();
        for (name, bytes) in case["files"].as_object().unwrap() {
            assert_eq!(
                fs::read_to_string(data.join(name)).unwrap(),
                bytes.as_str().unwrap()
            );
        }
        assert_eq!(fs::read_dir(data).unwrap().count(), 3);
    }
}

struct Fail(Stage);
impl StorageHooks for Fail {
    fn before(&self, stage: Stage, _path: &Path) -> io::Result<()> {
        if stage == self.0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("injected {stage:?}"),
            ));
        }
        Ok(())
    }
}

#[test]
fn failing_default_reads_use_empty_values_but_settings_are_injected_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let paths = DataPaths::new(root.path().join("data")).unwrap();
    let store = Storage::with_hooks(paths.clone(), Arc::new(Fail(Stage::Read)));
    let settings = json!({"customCallerSettings": [true, 2.5]});
    store
        .bootstrap(&root.path().join("defaults"), &settings)
        .unwrap();
    assert_eq!(
        fs::read_to_string(paths.file(FileKind::Templates)).unwrap(),
        "[]"
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(paths.file(FileKind::Settings)).unwrap())
            .unwrap(),
        settings
    );
}

#[test]
fn bootstrap_io_errors_propagate_without_publishing_partial_files() {
    for stage in [
        Stage::CreateDir,
        Stage::Exists,
        Stage::CreateTemp,
        Stage::Write,
        Stage::Sync,
        Stage::CreateOnly,
    ] {
        let root = tempfile::tempdir().unwrap();
        let paths = DataPaths::new(root.path().join("data")).unwrap();
        let store = Storage::with_hooks(paths.clone(), Arc::new(Fail(stage)));
        assert!(
            store
                .bootstrap(&root.path().join("defaults"), &json!({}))
                .unwrap_err()
                .to_string()
                .contains(&format!("{stage:?}"))
        );
        assert!(!paths.file(FileKind::Tasks).exists());
        if paths.data_dir().exists() {
            assert_eq!(fs::read_dir(paths.data_dir()).unwrap().count(), 0);
        }
    }
}

struct CompetingCreator;
impl StorageHooks for CompetingCreator {
    fn before(&self, stage: Stage, path: &Path) -> io::Result<()> {
        if stage == Stage::CreateOnly {
            fs::write(path, b"winner bytes")?;
        }
        Ok(())
    }
}
#[test]
fn create_only_publication_never_clobbers_a_file_created_after_the_exists_check() {
    let root = tempfile::tempdir().unwrap();
    let paths = DataPaths::new(root.path().join("data")).unwrap();
    let store = Storage::with_hooks(paths.clone(), Arc::new(CompetingCreator));
    store
        .bootstrap(&root.path().join("defaults"), &json!({}))
        .unwrap();
    for kind in [FileKind::Tasks, FileKind::Templates, FileKind::Settings] {
        assert_eq!(fs::read(paths.file(kind)).unwrap(), b"winner bytes");
    }
    assert_eq!(fs::read_dir(paths.data_dir()).unwrap().count(), 3);
}
