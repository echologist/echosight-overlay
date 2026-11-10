use echosight_core::task::{
    background::{
        BackgroundCommand as Command, BackgroundTaskState, BackgroundTransition, trigger_rows,
    },
    model::{BackgroundPriority, TaskEnvironment, TaskId},
    state::TaskState,
};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

struct Clock(f64);
impl TaskEnvironment for Clock {
    fn now_ms(&self) -> f64 {
        self.0
    }
    fn now_iso(&self) -> String {
        chrono::DateTime::from_timestamp_millis(self.0 as i64)
            .unwrap()
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }
    fn random(&mut self) -> f64 {
        0.017
    }
}

#[test]
fn background_controller_oracle() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/background");
    let mut inputs: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".input.json"))
        .collect();
    inputs.sort();
    assert_eq!(inputs.len(), 15);
    for input_path in inputs {
        let input: Value = serde_json::from_str(&fs::read_to_string(&input_path).unwrap()).unwrap();
        let expected_path = input_path.with_file_name(
            input_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .replace(".input.", ".expected."),
        );
        let expected_bytes = fs::read_to_string(expected_path).unwrap();
        let expected: Value = serde_json::from_str(&expected_bytes).unwrap();
        let mut actual_snapshots = Vec::new();
        let state = TaskState::new(
            serde_json::from_value(input["tasks"].clone()).unwrap(),
            None,
            vec![],
            Clock(input["epoch"].as_f64().unwrap()),
        );
        let mut background = BackgroundTaskState::new(state);
        for (i, op) in input["ops"].as_array().unwrap().iter().enumerate() {
            let id = || TaskId(op["id"].as_f64().unwrap());
            let mut metadata = Value::Null;
            let transition = match op["op"].as_str().unwrap() {
                "activate" => background.dispatch(Command::ActivateTriggers(
                    serde_json::from_value(op["triggers"].clone()).unwrap(),
                )),
                "deactivate" => background.dispatch(Command::DeactivateTriggers(
                    serde_json::from_value(op["triggers"].clone()).unwrap(),
                )),
                "capture" => {
                    background.dispatch(Command::CaptureUndo(op["label"].as_str().unwrap().into()))
                }
                "next" => background.dispatch(Command::CompleteNext),
                "toggle" => background.dispatch(Command::Toggle(id())),
                "restart" => background.dispatch(Command::Restart),
                "cancel" => background.dispatch(Command::Cancel(id())),
                "cancelAll" => background.dispatch(Command::CancelAll),
                "undo" => background.dispatch(Command::Undo),
                "redo" => background.dispatch(Command::Redo),
                "delete" => background.dispatch(Command::Delete(id())),
                "clear" => background.dispatch(Command::Clear),
                "save" => background.dispatch(Command::Save),
                "replace" => background.dispatch(Command::Replace {
                    tasks: serde_json::from_value(op["tasks"].clone()).unwrap(),
                    current_template: None,
                }),
                "addBackground" => background.dispatch(Command::AddBackground {
                    text: op["text"].as_str().unwrap().into(),
                    options: op["options"].clone(),
                }),
                "editDormant" => background.dispatch(Command::Edit {
                    id: id(),
                    text: op["text"].as_str().unwrap().into(),
                    priority: Some(if op["priority"] == "high" {
                        BackgroundPriority::High
                    } else {
                        BackgroundPriority::Normal
                    }),
                }),
                "configure" => {
                    metadata = serde_json::to_value(trigger_rows(
                        background.state().find_task(id()).unwrap(),
                        background.state().tasks(),
                    ))
                    .unwrap();
                    BackgroundTransition::default()
                }
                "triggers" => {
                    let triggers = background
                        .state()
                        .find_task(id())
                        .map(|task| {
                            trigger_rows(task, background.state().tasks())
                                .into_iter()
                                .filter(|row| {
                                    op["selected"]
                                        .as_array()
                                        .unwrap()
                                        .iter()
                                        .any(|v| v.as_f64() == Some(row.id.0))
                                })
                                .map(|row| row.id)
                                .collect()
                        })
                        .unwrap_or_default();
                    background.dispatch(Command::UpdateTriggers { id: id(), triggers })
                }
                "advance" => {
                    let until =
                        background.state().environment.now_ms() + op["ms"].as_f64().unwrap();
                    let mut result = BackgroundTransition::default();
                    while let Some(due) = background.next_due_ms().filter(|due| *due <= until) {
                        background.environment_mut().0 = due;
                        result
                            .effects
                            .extend(background.dispatch(Command::ExpireDue).effects);
                    }
                    background.environment_mut().0 = until;
                    result
                }
                unknown => panic!("unknown command {unknown}"),
            };
            let counted: usize = transition
                .effects
                .iter()
                .filter_map(|effect| {
                    if let echosight_core::task::background::BackgroundEffect::Activated {
                        count,
                        ..
                    } = effect
                    {
                        Some(*count)
                    } else {
                        None
                    }
                })
                .sum();
            assert_eq!(transition.activated_count, counted);
            let actual = json!({"nowMs":background.state().environment.now_ms() as i64,"tasks":background.state().tasks(),"timers":background.timers(),"effects":transition.effects,"metadata":metadata,"history":transition.history});
            assert_eq!(
                actual,
                expected["snapshots"][i],
                "{} operation {i}: {op}",
                input_path.display()
            );
            actual_snapshots.push(actual);
        }
        // Provenance is verified read-only by the TS oracle; here verify every
        // generated result plus JSON.stringify-compatible pretty bytes.
        let actual = json!({"provenance":expected["provenance"],"snapshots":actual_snapshots});
        assert_eq!(
            echosight_core::storage::json_format::json_format(&actual).unwrap(),
            expected_bytes,
            "{} pretty bytes",
            input_path.display()
        );
    }
}
