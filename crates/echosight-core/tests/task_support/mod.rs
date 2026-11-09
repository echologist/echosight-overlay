use echosight_core::task::{
    completion::has_incomplete_tasks,
    creation::sanitize_background_options,
    migration::migrate_task_structure,
    model::{BackgroundPriority, TaskEnvironment, TaskId},
    state::TaskState,
    tree::{find_parent_task, find_task_with_parent, flatten_tasks},
};
use serde_json::{Value, json};
use std::collections::HashMap;

struct Environment {
    ms: f64,
    iso: String,
    random_calls: u64,
}
impl TaskEnvironment for Environment {
    fn now_ms(&self) -> f64 {
        self.ms
    }
    fn now_iso(&self) -> String {
        self.iso.clone()
    }
    fn random(&mut self) -> f64 {
        let value = ((self.random_calls * 137 + 17) % 1000) as f64 / 1000.0;
        self.random_calls += 1;
        value
    }
}

pub fn execute(input: &Value) -> Value {
    match input["kind"].as_str().unwrap() {
        "migration" => {
            let mut tasks = input["tasks"].clone();
            let migrated = migrate_task_structure(&mut tasks);
            let second = migrate_task_structure(&mut tasks);
            json!({ "migrated": migrated, "tasks": tasks, "secondMigration": second })
        }
        "options" => Value::Array(
            input["values"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| json!(sanitize_background_options(v)))
                .collect(),
        ),
        "state" => run_state(input),
        kind => panic!("unknown fixture kind: {kind}"),
    }
}

fn run_state(input: &Value) -> Value {
    let mut state = TaskState::new(
        serde_json::from_value(input["tasks"].clone()).unwrap(),
        serde_json::from_value(input["currentTemplate"].clone()).unwrap(),
        input["snapshots"].as_array().unwrap().clone(),
        Environment {
            ms: input["epoch"].as_f64().unwrap(),
            iso: String::new(),
            random_calls: 0,
        },
    );
    let mut aliases: HashMap<String, TaskId> = HashMap::new();
    let mut frames = vec![];
    for op in input["ops"].as_array().unwrap() {
        state.environment.ms = op["nowMs"].as_f64().unwrap();
        state.environment.iso = op["nowIso"].as_str().unwrap().to_owned();
        let id = |value: &Value| -> TaskId {
            value
                .as_str()
                .map(|s| aliases[s])
                .unwrap_or_else(|| TaskId(value.as_f64().unwrap()))
        };
        let result = match op["op"].as_str().unwrap() {
            "add" => {
                let parent = if op["parent"].is_null() {
                    None
                } else {
                    Some(id(&op["parent"]))
                };
                let result = state.add_task(op["text"].as_str().unwrap().to_owned(), parent);
                if let (Some(result), Some(alias)) = (&result, op["as"].as_str()) {
                    aliases.insert(alias.to_owned(), result.task.id);
                }
                json!(result)
            }
            "background" => {
                let task = state
                    .add_background_task(op["text"].as_str().unwrap().to_owned(), &op["options"]);
                if let Some(alias) = op["as"].as_str() {
                    aliases.insert(alias.to_owned(), task.id);
                }
                json!(task)
            }
            "edit" => {
                let priority: Option<BackgroundPriority> =
                    serde_json::from_value(op["priority"].clone()).unwrap();
                json!(state.edit_task(id(&op["id"]), op["text"].as_str().unwrap(), priority))
            }
            "delete" => json!(state.delete_task(id(&op["id"]))),
            "toggle" => json!(state.toggle_task(id(&op["id"]))),
            "next" => json!(state.complete_next_task()),
            "undo" => json!(state.undo()),
            "redo" => json!(state.redo()),
            "clear" => {
                state.clear_tasks();
                Value::Null
            }
            "capture" => {
                state.capture_undo_state(op["label"].as_str().unwrap());
                Value::Null
            }
            "replace" => {
                state.replace_tasks(
                    serde_json::from_value(op["tasks"].clone()).unwrap(),
                    serde_json::from_value(op["currentTemplate"].clone()).unwrap(),
                );
                Value::Null
            }
            "move" => json!(state.move_task(
                id(&op["id"]),
                id(&op["target"]),
                op["above"].as_bool().unwrap_or(false),
                op["subtask"].as_bool().unwrap_or(false)
            )),
            "find" => json!(state.find_task(id(&op["id"]))),
            "parent" => json!(find_parent_task(id(&op["id"]), state.tasks())),
            "location" => find_task_with_parent(id(&op["id"]), state.tasks())
                .map(|l| {
                    json!({
                        "task": l.task, "parentId": l.parent.map(|p| p.id), "level": l.level,
                        "arrayIds": l.array.iter().map(|t| t.id).collect::<Vec<_>>()
                    })
                })
                .unwrap_or(Value::Null),
            "flatten" => json!(
                flatten_tasks(state.tasks())
                    .iter()
                    .map(|t| t.id)
                    .collect::<Vec<_>>()
            ),
            "incomplete" => json!(has_incomplete_tasks(state.tasks())),
            "progress" => json!(state.progress()),
            "save" => json!(state.record_auto_save_snapshot()),
            unknown => panic!("unknown operation: {unknown}"),
        };
        frames.push(
            json!({ "result": result, "tasks": state.tasks(), "progress": state.progress() }),
        );
    }
    Value::Array(frames)
}

pub fn first_difference(actual: &Value, expected: &Value, path: &str) -> String {
    match (actual, expected) {
        (Value::Array(a), Value::Array(e)) if a.len() == e.len() => {
            for (i, (a, e)) in a.iter().zip(e).enumerate() {
                if a != e {
                    return first_difference(a, e, &format!("{path}[{i}]"));
                }
            }
        }
        (Value::Object(a), Value::Object(e)) => {
            for (k, e) in e {
                let a = a.get(k).unwrap_or(&Value::Null);
                if a != e {
                    return first_difference(a, e, &format!("{path}.{k}"));
                }
            }
        }
        _ => {}
    }
    format!("{path}: actual {actual}, expected {expected}")
}
