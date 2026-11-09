use super::model::{Task, TaskMode, serialize_number};
use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct TaskProgress {
    pub completed: usize,
    pub total: usize,
    #[serde(serialize_with = "serialize_number")]
    pub percentage: f64,
}

pub fn calculate_task_progress(tasks: &[Task]) -> TaskProgress {
    fn count(tasks: &[Task], completed: &mut usize, total: &mut usize) {
        for task in tasks {
            count(&task.children, completed, total);
            *completed += usize::from(task.completed);
            *total += 1;
        }
    }
    let (mut completed, mut total) = (0, 0);
    // TS filters ROOTS only. Dormant descendants of included roots still count.
    for task in tasks
        .iter()
        .filter(|t| t.mode != TaskMode::Background || t.activated)
    {
        count(std::slice::from_ref(task), &mut completed, &mut total);
    }
    TaskProgress {
        completed,
        total,
        percentage: if total > 0 {
            completed as f64 / total as f64 * 100.0
        } else {
            0.0
        },
    }
}
