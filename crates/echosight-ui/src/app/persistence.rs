use echosight_core::{
    storage::{DataPaths, FileKind, Storage},
    task::{migration::migrate_task_structure, model::TaskSaveData},
};
use serde_json::{Value, json};

/// Blocking adapter: call only on the background executor. The missing-file
/// fallback is embedded; neither startup nor a failed typed decode writes data.
/// First user mutation persists the defaults through the same FIFO as all edits.
pub(super) fn load(storage: &Storage, paths: &DataPaths) -> Result<TaskSaveData, String> {
    let missing = match std::fs::symlink_metadata(paths.file(FileKind::Tasks)) {
        Ok(_) => false,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
        Err(e) => return Err(e.to_string()),
    };
    let mut raw = storage.load(FileKind::Tasks).map_err(|e| e.to_string())?;
    if missing {
        raw = serde_json::from_str(include_str!("../../../../data/current_tasks.json"))
            .map_err(|e| e.to_string())?;
    }
    if let Some(backup) = raw.get("corruptBackupPath") {
        // Recovery is performed by reviewed Storage. Require an explicit restart
        // before enabling edits, so a recovery notice cannot silently disappear.
        return Err(format!(
            "Corrupt tasks preserved at {backup}. Restart to use recovered data."
        ));
    }
    let object = raw.as_object_mut().ok_or("Task file must be an object")?;
    object.entry("currentTemplate").or_insert(Value::Null);
    object.entry("snapshots").or_insert(json!([]));
    let tasks = object
        .get_mut("tasks")
        .ok_or("Task file is missing tasks")?;
    migrate_task_structure(tasks);
    serde_json::from_value(raw).map_err(|e| format!("Unsupported task data; writes blocked: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_directory_loads_embedded_tasks_without_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::new(dir.path().join("development")).unwrap();
        let loaded = load(&Storage::new(paths.clone()), &paths).unwrap();
        assert_eq!(loaded.tasks[0].text, "Hello there! :)");
        assert!(!paths.data_dir().exists());
    }
}
