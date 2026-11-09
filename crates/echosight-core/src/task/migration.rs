use crate::jscompat::js_truthy;
use serde_json::{Value, json};

/// Migrate raw JSON in place, preserving key order, unknown fields and ignored
/// non-record entries exactly. This is not validation and does not coerce IDs.
pub fn migrate_task_structure(tasks: &mut Value) -> bool {
    let Some(tasks) = tasks.as_array_mut() else {
        return false;
    };
    let mut migrated = false;
    for task in tasks {
        let Some(task) = task.as_object_mut() else {
            continue;
        };
        if !task.get("children").is_some_and(Value::is_array) {
            task.insert("children".into(), json!([]));
            migrated = true;
        }
        if !task.get("mode").is_some_and(js_truthy) {
            task.insert("mode".into(), json!("main"));
            migrated = true;
        }
        if !task.get("triggers").is_some_and(Value::is_array) {
            task.insert("triggers".into(), json!([]));
            migrated = true;
        }
        for (key, default) in [
            ("activated", json!(true)),
            ("activatedAt", Value::Null),
            ("backgroundOptions", Value::Null),
        ] {
            if !task.contains_key(key) {
                task.insert(key.into(), default);
                migrated = true;
            }
        }
        migrated |= migrate_task_structure(task.get_mut("children").expect("children initialized"));
    }
    migrated
}
