use echosight_core::task::{
    background::{BackgroundCommand as Command, BackgroundEffect, BackgroundTaskState},
    model::{Task, TaskEnvironment, TaskId},
    state::TaskState,
};
use serde_json::json;

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

fn state() -> TaskState<Clock> {
    let task = |id, children| {
        json!({"id": id, "text": format!("Task {id}"),
        "completed": false, "createdAt": "2027-01-15T08:00:00.000Z", "children": children, "mode": "background", "activated": true,
        "backgroundOptions": {"expiresAfterMinutes": null, "priority": "normal"}})
    };
    let mut child = task(20, json!([]));
    child["backgroundOptions"]["expiresAfterMinutes"] = json!(1);
    let tasks: Vec<Task> =
        serde_json::from_value(json!([task(10, json!([])), task(10, json!([child]))])).unwrap();
    TaskState::new(tasks, None, vec![], Clock(1_800_000_000_000.))
}

#[test]
fn failed_move_restores_original_structural_parent() {
    let mut state = state();
    let before = state.tasks().to_vec();
    let result = state.move_task(TaskId(20.), TaskId(20.), true, false);
    assert!(!result.success);
    assert_eq!(
        result.reason.as_deref(),
        Some("Could not find drop target 20")
    );
    assert_eq!(state.tasks(), before);
}

#[test]
fn failed_move_preserves_captured_expiry_under_duplicate_ancestor() {
    let mut background = BackgroundTaskState::new(state());
    background.dispatch(Command::Restart);
    let due = background.next_due_ms().unwrap();
    let result = background.dispatch(Command::Move {
        id: TaskId(20.),
        target: TaskId(20.),
        above: true,
        subtask: false,
    });
    assert!(!result.moved.unwrap().success);
    assert!(result.effects.is_empty());
    assert_eq!(background.next_due_ms(), Some(due));
    background.environment_mut().0 = due;
    let result = background.dispatch(Command::ExpireDue);
    assert!(background.state().tasks()[0].children.is_empty());
    assert!(!background.state().tasks()[1].children[0].activated);
    assert_eq!(background.next_due_ms(), None);
    assert!(
        matches!(&result.effects[..], [BackgroundEffect::Persist { .. }, BackgroundEffect::Log { message }, BackgroundEffect::Cancel { id }] if message == "Background task \"Task 20\" expired" && *id == TaskId(20.))
    );
}

#[test]
fn move_destination_retains_duplicate_parent_identity() {
    let mut state = state();
    let mut tasks = state.tasks().to_vec();
    let mut source = tasks[1].children[0].clone();
    source.id = TaskId(30.);
    tasks.insert(0, source); // Removal also shifts the target parent's root index.
    state.replace_tasks(tasks, None);
    let result = state.move_task(TaskId(30.), TaskId(20.), true, false);
    assert!(result.success);
    assert!(state.tasks()[0].children.is_empty());
    assert_eq!(
        state.tasks()[1]
            .children
            .iter()
            .map(|t| t.id)
            .collect::<Vec<_>>(),
        vec![TaskId(30.), TaskId(20.)]
    );
}
