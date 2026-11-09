use super::{
    model::{Task, TaskId, TaskMode},
    tree::{find_parent_task, find_task_by_id, find_task_by_id_mut, flatten_tasks},
};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteTaskResult {
    pub task: Task,
    pub completed_parents: Vec<Task>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToggleTaskResult {
    pub task: Task,
    pub completed_parents: Vec<Task>,
    pub unchecked_parents: Vec<Task>,
}

pub fn has_incomplete_tasks(tasks: &[Task]) -> bool {
    flatten_tasks(tasks).iter().any(|t| !t.completed)
}

pub fn complete_all_children(parent: &mut Task) {
    for child in &mut parent.children {
        child.completed = true;
        complete_all_children(child);
    }
}

pub fn check_parent_completion(child_id: TaskId, tasks: &mut [Task]) -> Vec<Task> {
    let mut completed = Vec::new();
    let mut id = child_id;
    while let Some(parent) = find_parent_task(id, tasks) {
        if parent.completed || !parent.children.iter().all(|c| c.completed) {
            break;
        }
        id = parent.id;
        let parent = find_task_by_id_mut(id, tasks).expect("found parent");
        parent.completed = true;
        completed.push(id);
    }
    completed
        .into_iter()
        .map(|id| find_task_by_id(id, tasks).expect("found parent").clone())
        .collect()
}

pub fn uncheck_parent(child_id: TaskId, tasks: &mut [Task]) -> Vec<Task> {
    let mut unchecked = Vec::new();
    let mut id = child_id;
    while let Some(parent) = find_parent_task(id, tasks) {
        if !parent.completed {
            break;
        }
        id = parent.id;
        find_task_by_id_mut(id, tasks)
            .expect("found parent")
            .completed = false;
        unchecked.push(id);
    }
    unchecked
        .into_iter()
        .map(|id| find_task_by_id(id, tasks).expect("found parent").clone())
        .collect()
}

pub fn find_next_incomplete_leaf_task(tasks: &[Task]) -> Option<&Task> {
    find_next_incomplete_leaf_with_path(tasks).map(|(_, task)| task)
}

fn find_next_incomplete_leaf_with_path(tasks: &[Task]) -> Option<(Vec<usize>, &Task)> {
    for (index, task) in tasks.iter().enumerate() {
        if task.completed {
            continue;
        }
        if !task.children.is_empty() {
            if let Some((mut path, leaf)) = find_next_incomplete_leaf_with_path(&task.children) {
                path.insert(0, index);
                return Some((path, leaf));
            }
            continue;
        }
        if task.mode != TaskMode::Background {
            return Some((vec![index], task));
        }
    }
    None
}

fn task_at_path_mut<'a>(mut tasks: &'a mut [Task], path: &[usize]) -> &'a mut Task {
    let (index, ancestors) = path.split_last().expect("selected leaf has a path");
    for &ancestor in ancestors {
        tasks = &mut tasks[ancestor].children;
    }
    &mut tasks[*index]
}

pub fn complete_next_task_in_list(tasks: &mut [Task]) -> Option<CompleteTaskResult> {
    // Root filtering is separate from the recursive leaf search in TS.
    let (path, id) = tasks
        .iter()
        .enumerate()
        .filter(|(_, t)| t.mode != TaskMode::Background)
        .find_map(|(index, t)| {
            let (mut path, leaf) = find_next_incomplete_leaf_with_path(std::slice::from_ref(t))?;
            path[0] = index;
            Some((path, leaf.id))
        })?;
    // TS retains the selected object, not its potentially colliding ID. Parent
    // completion still resolves by ID; it changes flags but never this path.
    task_at_path_mut(tasks, &path).completed = true;
    let completed_parents = check_parent_completion(id, tasks);
    Some(CompleteTaskResult {
        task: task_at_path_mut(tasks, &path).clone(),
        completed_parents,
    })
}

pub fn toggle_task_in_list(id: TaskId, tasks: &mut [Task]) -> Option<ToggleTaskResult> {
    let task = find_task_by_id_mut(id, tasks)?;
    task.completed = !task.completed;
    let completed = task.completed;
    if completed {
        complete_all_children(task);
    }
    let completed_parents = if completed {
        check_parent_completion(id, tasks)
    } else {
        vec![]
    };
    let unchecked_parents = if completed {
        vec![]
    } else {
        uncheck_parent(id, tasks)
    };
    Some(ToggleTaskResult {
        task: find_task_by_id(id, tasks)?.clone(),
        completed_parents,
        unchecked_parents,
    })
}
