use super::{
    completion::{
        CompleteTaskResult, ToggleTaskResult, complete_next_task_in_list, toggle_task_in_list,
    },
    creation::create_background_task,
    model::{BackgroundPriority, Task, TaskEnvironment, TaskId, TaskSaveData, TaskStateSnapshot},
    mutations::{
        AddTaskResult, DeleteTaskResult, MoveTaskResult, add_task_to_list, delete_task_from_list,
        edit_task_in_list, move_task_in_tree,
    },
    progress::{TaskProgress, calculate_task_progress},
    tree::find_task_by_id,
};
use serde::Serialize;
use serde_json::{Value, json};

pub const MAX_UNDO_ACTIONS: usize = 20;
pub const MAX_AUTO_SAVE_SNAPSHOTS: usize = 5;

#[derive(Clone, Debug)]
struct UndoEntry {
    label: String,
    state: TaskStateSnapshot,
    // Retain the source's capture time, although the public TS API doesn't expose it.
    _created_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct UndoTaskActionResult {
    pub restored: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Pure counterpart of taskStateController. No I/O, timers or sound effects.
/// Loaded data is supplied by the caller; `record_auto_save_snapshot` creates the
/// exact payload before an adapter attempts persistence (even if that save fails).
pub struct TaskState<E> {
    state: TaskStateSnapshot,
    snapshots: Vec<Value>,
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
    pub environment: E,
}

impl<E: TaskEnvironment> TaskState<E> {
    pub fn new(
        tasks: Vec<Task>,
        current_template: Option<String>,
        mut snapshots: Vec<Value>,
        environment: E,
    ) -> Self {
        cap(&mut snapshots, MAX_AUTO_SAVE_SNAPSHOTS);
        Self {
            state: TaskStateSnapshot {
                tasks,
                current_template,
            },
            snapshots,
            undo_stack: vec![],
            redo_stack: vec![],
            environment,
        }
    }

    pub fn tasks(&self) -> &[Task] {
        &self.state.tasks
    }
    pub fn current_template(&self) -> Option<&str> {
        self.state.current_template.as_deref()
    }
    pub fn find_task(&self, id: TaskId) -> Option<&Task> {
        find_task_by_id(id, self.tasks())
    }
    pub fn progress(&self) -> TaskProgress {
        calculate_task_progress(self.tasks())
    }

    /// Like TS replaceTasks, this is not implicitly an undoable action.
    pub fn replace_tasks(&mut self, tasks: Vec<Task>, current_template: Option<String>) {
        self.state = TaskStateSnapshot {
            tasks,
            current_template,
        };
    }

    pub fn capture_undo_state(&mut self, label: &str) {
        self.push_undo(label, self.state.clone());
    }

    pub fn clear_tasks(&mut self) {
        if self.state.tasks.is_empty() && self.state.current_template.is_none() {
            return;
        }
        self.capture_undo_state("clear all tasks");
        self.replace_tasks(vec![], None);
    }

    pub fn add_task(&mut self, text: String, parent: Option<TaskId>) -> Option<AddTaskResult> {
        let before = self.state.clone();
        let result = add_task_to_list(&mut self.state.tasks, text, parent, &mut self.environment);
        if result.is_some() {
            self.push_undo("add task", before);
        }
        result
    }

    pub fn add_background_task(&mut self, text: String, options: &Value) -> Task {
        let before = self.state.clone();
        let task = create_background_task(text, options, &mut self.environment);
        self.state.tasks.push(task.clone());
        self.push_undo("add background task", before);
        task
    }

    pub fn delete_task(&mut self, id: TaskId) -> DeleteTaskResult {
        let before = self.state.clone();
        let result = delete_task_from_list(id, &mut self.state.tasks);
        if result.removed {
            self.push_undo("delete task", before);
        }
        result
    }

    pub fn edit_task(
        &mut self,
        id: TaskId,
        text: &str,
        priority: Option<BackgroundPriority>,
    ) -> bool {
        let before = self.state.clone();
        let changed = edit_task_in_list(id, &mut self.state.tasks, text, priority);
        if changed {
            self.push_undo("edit task", before);
        }
        changed
    }

    pub fn move_task(
        &mut self,
        id: TaskId,
        target: TaskId,
        above: bool,
        subtask: bool,
    ) -> MoveTaskResult {
        let before = self.state.clone();
        let result = move_task_in_tree(&mut self.state.tasks, id, target, above, subtask);
        if result.success {
            self.push_undo("move task", before);
        }
        result
    }

    pub fn toggle_task(&mut self, id: TaskId) -> Option<ToggleTaskResult> {
        let before = self.state.clone();
        let result = toggle_task_in_list(id, &mut self.state.tasks);
        if result.is_some() {
            self.push_undo("toggle task", before);
        }
        result
    }

    pub fn complete_next_task(&mut self) -> Option<CompleteTaskResult> {
        let before = self.state.clone();
        let result = complete_next_task_in_list(&mut self.state.tasks);
        if result.is_some() {
            self.push_undo("complete next task", before);
        }
        result
    }

    pub fn undo(&mut self) -> UndoTaskActionResult {
        let Some(entry) = self.undo_stack.pop() else {
            return UndoTaskActionResult {
                restored: false,
                label: None,
            };
        };
        self.redo_stack
            .push(self.history_entry(&entry.label, self.state.clone()));
        cap(&mut self.redo_stack, MAX_UNDO_ACTIONS);
        self.state = entry.state;
        UndoTaskActionResult {
            restored: true,
            label: Some(entry.label),
        }
    }

    pub fn redo(&mut self) -> UndoTaskActionResult {
        let Some(entry) = self.redo_stack.pop() else {
            return UndoTaskActionResult {
                restored: false,
                label: None,
            };
        };
        self.undo_stack
            .push(self.history_entry(&entry.label, self.state.clone()));
        cap(&mut self.undo_stack, MAX_UNDO_ACTIONS);
        self.state = entry.state;
        UndoTaskActionResult {
            restored: true,
            label: Some(entry.label),
        }
    }

    pub fn record_auto_save_snapshot(&mut self) -> TaskSaveData {
        self.snapshots
            .push(json!({ "createdAt": self.environment.now_iso(), "state": self.state }));
        cap(&mut self.snapshots, MAX_AUTO_SAVE_SNAPSHOTS);
        TaskSaveData {
            tasks: self.state.tasks.clone(),
            current_template: self.state.current_template.clone(),
            snapshots: self.snapshots.clone(),
        }
    }

    fn history_entry(&self, label: &str, state: TaskStateSnapshot) -> UndoEntry {
        UndoEntry {
            label: label.to_owned(),
            state,
            _created_at: self.environment.now_iso(),
        }
    }

    fn push_undo(&mut self, label: &str, state: TaskStateSnapshot) {
        self.undo_stack.push(self.history_entry(label, state));
        cap(&mut self.undo_stack, MAX_UNDO_ACTIONS);
        self.redo_stack.clear();
    }
}

fn cap<T>(items: &mut Vec<T>, limit: usize) {
    if items.len() > limit {
        items.drain(..items.len() - limit);
    }
}
