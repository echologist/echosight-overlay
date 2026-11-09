use super::{
    creation::create_task,
    model::{BackgroundOptions, BackgroundPriority, Task, TaskEnvironment, TaskId, TaskMode},
    tree::{
        collect_task_ids, find_task_by_id, find_task_by_id_mut, find_task_with_parent,
        remove_task_from_parent,
    },
};
use crate::jscompat::js_trim;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddTaskResult {
    pub task: Task,
    pub parent_task: Option<Task>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTaskResult {
    pub removed: bool,
    pub removed_task_ids: Vec<TaskId>,
}

pub fn add_task_to_list(
    tasks: &mut Vec<Task>,
    text: String,
    parent_id: Option<TaskId>,
    env: &mut impl TaskEnvironment,
) -> Option<AddTaskResult> {
    let parent = match parent_id {
        Some(id) => Some(find_task_by_id_mut(id, tasks)?),
        None => None,
    };
    let task = create_task(text, parent.as_deref(), env);
    let parent_task = if let Some(parent) = parent {
        parent.children.push(task.clone());
        Some(parent.clone())
    } else {
        tasks.push(task.clone());
        None
    };
    Some(AddTaskResult { task, parent_task })
}

pub fn delete_task_from_list(id: TaskId, tasks: &mut Vec<Task>) -> DeleteTaskResult {
    let removed_task_ids = find_task_by_id(id, tasks)
        .map(collect_task_ids)
        .unwrap_or_else(|| vec![id]);
    fn strip(tasks: &mut [Task], ids: &[TaskId]) {
        for task in tasks {
            task.triggers.retain(|id| !ids.contains(id));
            strip(&mut task.children, ids);
        }
    }
    // Even missing IDs strip dangling references; the controller does NOT record
    // history unless an actual task was removed. Preserve that source quirk.
    strip(tasks, &removed_task_ids);
    DeleteTaskResult {
        removed: remove_task_from_parent(id, tasks).is_some(),
        removed_task_ids,
    }
}

pub fn edit_task_in_list(
    id: TaskId,
    tasks: &mut [Task],
    text: &str,
    priority: Option<BackgroundPriority>,
) -> bool {
    let text = js_trim(text);
    if text.is_empty() {
        return false;
    }
    let Some(task) = find_task_by_id_mut(id, tasks) else {
        return false;
    };
    let priority_changed = task.mode == TaskMode::Background
        && priority.is_some_and(|p| {
            p != task
                .background_options
                .as_ref()
                .map(|o| o.priority)
                .unwrap_or_default()
        });
    if text == task.text && !priority_changed {
        return false;
    }
    task.text = text.to_owned();
    if priority_changed {
        task.background_options = Some(BackgroundOptions {
            expires_after_minutes: task
                .background_options
                .as_ref()
                .and_then(|o| o.expires_after_minutes),
            priority: priority.expect("changed priority supplied"),
        });
    }
    true
}

#[derive(Clone, Debug, Serialize)]
pub struct MoveTaskResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl MoveTaskResult {
    fn failure(reason: String) -> Self {
        Self {
            success: false,
            operation: None,
            reason: Some(reason),
        }
    }
}

pub fn move_task_in_tree(
    tasks: &mut Vec<Task>,
    dragged_id: TaskId,
    target_id: TaskId,
    insert_above: bool,
    make_subtask: bool,
) -> MoveTaskResult {
    let Some(dragged) = find_task_with_parent(dragged_id, tasks) else {
        return MoveTaskResult::failure(format!("Task not found: {dragged_id}"));
    };
    let Some(target) = find_task_with_parent(target_id, tasks) else {
        return MoveTaskResult::failure(format!("Task not found: {target_id}"));
    };
    if dragged.task.mode != target.task.mode {
        return MoveTaskResult::failure("Cannot reorder across main/background lists".into());
    }
    if find_task_by_id(target_id, &dragged.task.children).is_some() {
        return MoveTaskResult::failure("Cannot move a task into its own subtree".into());
    }
    let operation = if make_subtask {
        format!(
            "Made \"{}\" a subtask of \"{}\"",
            dragged.task.text, target.task.text
        )
    } else if dragged.level == target.level {
        format!("Reordered \"{}\" within same level", dragged.task.text)
    } else if dragged.level > target.level {
        format!(
            "Promoted \"{}\" from level {} to level {}",
            dragged.task.text, dragged.level, target.level
        )
    } else {
        format!(
            "Moved \"{}\" to be sibling of \"{}\"",
            dragged.task.text, target.task.text
        )
    };
    let original_parent = dragged.parent.map(|p| p.id);
    let original_index = dragged
        .array
        .iter()
        .position(|t| t.id == dragged_id)
        .expect("found task");
    let target_parent = target.parent.map(|p| p.id);
    let task = remove_task_from_parent(dragged_id, tasks).expect("found task");
    // getTaskDropOperation prevents self-drops in the UI, but the lower-level
    // TS function accepts self-subtask and loses the detached task. Match it.
    if make_subtask && dragged_id == target_id {
        return MoveTaskResult {
            success: true,
            operation: Some(operation),
            reason: None,
        };
    }
    let destination = if make_subtask {
        find_task_by_id_mut(target_id, tasks).map(|t| {
            let i = t.children.len();
            (&mut t.children, i)
        })
    } else {
        let array = array_for_parent(tasks, target_parent);
        array
            .iter()
            .position(|t| t.id == target_id)
            .map(|i| (array, i + usize::from(!insert_above)))
    };
    let Some((array, index)) = destination else {
        let array = array_for_parent(tasks, original_parent);
        array.insert(original_index.min(array.len()), task);
        return MoveTaskResult::failure(format!("Could not find drop target {target_id}"));
    };
    array.insert(index, task);
    MoveTaskResult {
        success: true,
        operation: Some(operation),
        reason: None,
    }
}

fn array_for_parent(tasks: &mut Vec<Task>, parent: Option<TaskId>) -> &mut Vec<Task> {
    match parent {
        Some(id) => {
            &mut find_task_by_id_mut(id, tasks)
                .expect("parent survives guarded move")
                .children
        }
        None => tasks,
    }
}
