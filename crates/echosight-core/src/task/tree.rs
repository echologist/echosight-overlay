use super::model::{Task, TaskId};

pub fn find_task_by_id(id: TaskId, tasks: &[Task]) -> Option<&Task> {
    for task in tasks {
        if task.id == id {
            return Some(task);
        }
        if let Some(found) = find_task_by_id(id, &task.children) {
            return Some(found);
        }
    }
    None
}

pub fn find_task_by_id_mut(id: TaskId, tasks: &mut [Task]) -> Option<&mut Task> {
    for task in tasks {
        if task.id == id {
            return Some(task);
        }
        if let Some(found) = find_task_by_id_mut(id, &mut task.children) {
            return Some(found);
        }
    }
    None
}

pub fn flatten_tasks(tasks: &[Task]) -> Vec<&Task> {
    let mut flat = Vec::new();
    for task in tasks {
        flat.push(task);
        flat.extend(flatten_tasks(&task.children));
    }
    flat
}

pub fn collect_task_ids(task: &Task) -> Vec<TaskId> {
    flatten_tasks(std::slice::from_ref(task))
        .into_iter()
        .map(|t| t.id)
        .collect()
}

pub fn find_parent_task(child_id: TaskId, tasks: &[Task]) -> Option<&Task> {
    for task in tasks {
        if task.children.iter().any(|child| child.id == child_id) {
            return Some(task);
        }
        if let Some(found) = find_parent_task(child_id, &task.children) {
            return Some(found);
        }
    }
    None
}

pub struct TaskLocation<'a> {
    pub task: &'a Task,
    pub parent: Option<&'a Task>,
    pub level: usize,
    pub array: &'a [Task],
}

pub fn find_task_with_parent(id: TaskId, tasks: &[Task]) -> Option<TaskLocation<'_>> {
    fn find<'a>(
        id: TaskId,
        tasks: &'a [Task],
        parent: Option<&'a Task>,
        level: usize,
    ) -> Option<TaskLocation<'a>> {
        for task in tasks {
            if task.id == id {
                return Some(TaskLocation {
                    task,
                    parent,
                    level,
                    array: tasks,
                });
            }
            if let Some(found) = find(id, &task.children, Some(task), level + 1) {
                return Some(found);
            }
        }
        None
    }
    find(id, tasks, None, 0)
}

pub fn remove_task_from_parent(id: TaskId, tasks: &mut Vec<Task>) -> Option<Task> {
    for i in 0..tasks.len() {
        if tasks[i].id == id {
            return Some(tasks.remove(i));
        }
        if let Some(found) = remove_task_from_parent(id, &mut tasks[i].children) {
            return Some(found);
        }
    }
    None
}
