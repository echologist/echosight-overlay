//! Background/trigger workflow with a pure, ID-keyed expiry scheduler.
//!
//! Dispatch commands on the UI thread and execute returned effects in order.
//! `Persist` owns the exact save payload at the point of the source callback;
//! storage, rendering, notifications, sounds and actual wakeups remain shell work.
//! There are no timers, threads or I/O here. Wake at `next_due_ms`, update the
//! injected clock, then dispatch `ExpireDue`. Never infer object identity from IDs.
use super::{
    model::{BackgroundPriority, Task, TaskEnvironment, TaskId, TaskMode, TaskSaveData},
    mutations::{MoveTaskResult, move_task_in_tree},
    state::{TaskState, UndoTaskActionResult},
    tree::flatten_tasks,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug)]
pub enum BackgroundCommand {
    CompleteNext,
    Toggle(TaskId),
    ActivateTriggers(Vec<TaskId>),
    DeactivateTriggers(Vec<TaskId>),
    Restart,
    Cancel(TaskId),
    CancelAll,
    ExpireDue,
    Undo,
    Redo,
    /// Confirmation belongs to the shell, before dispatch.
    Delete(TaskId),
    /// Like the workflow, ignores a list containing only dormant roots.
    Clear,
    /// Raw state replacement retains captured old timer targets, like TS.
    /// Template replacement must dispatch CancelAll first.
    Replace {
        tasks: Vec<Task>,
        current_template: Option<String>,
    },
    Edit {
        id: TaskId,
        text: String,
        priority: Option<BackgroundPriority>,
    },
    UpdateTriggers {
        id: TaskId,
        triggers: Vec<TaskId>,
    },
    /// Like taskState.addBackgroundTask, callers explicitly Save afterwards.
    AddBackground {
        text: String,
        options: Value,
    },
    AddTask {
        text: String,
        parent: Option<TaskId>,
    },
    Move {
        id: TaskId,
        target: TaskId,
        above: bool,
        subtask: bool,
    },
    CaptureUndo(String),
    Save,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BackgroundEffect {
    Schedule {
        id: TaskId,
        #[serde(rename = "dueMs", serialize_with = "super::model::serialize_number")]
        due_ms: f64,
    },
    Cancel {
        id: TaskId,
    },
    /// Also requests a task render and progress refresh. Already snapshot-capped;
    /// pass this payload to storage without recording a second auto-save snapshot.
    Persist {
        data: TaskSaveData,
    },
    Activated {
        count: usize,
        message: String,
    },
    Sound {
        event: &'static str,
    },
    Log {
        message: String,
    },
}

#[derive(Clone, Debug, Default)]
pub struct BackgroundTransition {
    /// Actual structurally selected completion; never re-search by ID in the shell.
    pub completed_task: Option<Task>,
    pub activated_count: usize,
    pub history: Option<UndoTaskActionResult>,
    pub moved: Option<MoveTaskResult>,
    pub effects: Vec<BackgroundEffect>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpirationTimer {
    pub id: TaskId,
    #[serde(serialize_with = "super::model::serialize_number")]
    pub due_ms: f64,
}

#[derive(Clone, Debug)]
struct PendingTimer {
    timer: ExpirationTimer,
    // A structural identity, not findTaskById: duplicate IDs overwrite the
    // timer map but its closure still captures the LAST scheduled object.
    path: Option<Vec<usize>>,
    detached: Task,
    sequence: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerRow {
    pub id: TaskId,
    pub text: String,
    pub checked: bool,
    pub priority_badge: Option<&'static str>,
    pub status_badge: &'static str,
    pub editable: bool,
}

pub fn trigger_rows(task: &Task, tasks: &[Task]) -> Vec<TriggerRow> {
    flatten_tasks(tasks)
        .into_iter()
        .filter(|t| t.mode == TaskMode::Background)
        .map(|t| TriggerRow {
            id: t.id,
            text: t.text.clone(),
            checked: task.triggers.contains(&t.id),
            priority_badge: t
                .background_options
                .as_ref()
                .filter(|o| o.priority == BackgroundPriority::High)
                .map(|_| "HIGH"),
            status_badge: if t.activated { "ACTIVE" } else { "DORMANT" },
            editable: true,
        })
        .collect()
}

/// Owns the task state so scheduled structural identities cannot silently become
/// stale through an unrelated mutable state reference. Existing `TaskState` stays
/// usable on its own for callers which do not want workflow side effects.
pub struct BackgroundTaskState<E> {
    state: TaskState<E>,
    timers: Vec<PendingTimer>,
    sequence: u64,
}

impl<E: TaskEnvironment> BackgroundTaskState<E> {
    pub fn new(state: TaskState<E>) -> Self {
        Self {
            state,
            timers: vec![],
            sequence: 0,
        }
    }
    pub fn state(&self) -> &TaskState<E> {
        &self.state
    }
    pub fn environment_mut(&mut self) -> &mut E {
        &mut self.state.environment
    }
    pub fn timers(&self) -> Vec<ExpirationTimer> {
        let mut timers: Vec<_> = self.timers.iter().map(|t| t.timer.clone()).collect();
        timers.sort_by(|a, b| a.id.0.total_cmp(&b.id.0));
        timers
    }
    pub fn next_due_ms(&self) -> Option<f64> {
        self.timers
            .iter()
            .map(|t| t.timer.due_ms)
            .min_by(f64::total_cmp)
    }
    pub fn dispatch(&mut self, command: BackgroundCommand) -> BackgroundTransition {
        let mut out = BackgroundTransition::default();
        match command {
            BackgroundCommand::CompleteNext => {
                if let Some(result) = self.state.complete_next_task() {
                    self.activate(&result.task.triggers, &mut out);
                    out.completed_task = Some(result.task);
                    for parent in result.completed_parents {
                        self.activate(&parent.triggers, &mut out);
                    }
                    self.persist(&mut out);
                    out.effects.push(BackgroundEffect::Sound {
                        event: "taskCompleted",
                    });
                }
            }
            BackgroundCommand::Toggle(id) => {
                if let Some(result) = self.state.toggle_task(id) {
                    if result.task.completed {
                        self.activate(&result.task.triggers, &mut out);
                        for parent in result.completed_parents {
                            self.activate(&parent.triggers, &mut out);
                        }
                        out.effects.push(BackgroundEffect::Sound {
                            event: "taskCompleted",
                        });
                    } else {
                        self.deactivate(&result.task.triggers, &mut out);
                        for parent in result.unchecked_parents {
                            self.deactivate(&parent.triggers, &mut out);
                        }
                    }
                    self.persist(&mut out);
                }
            }
            BackgroundCommand::ActivateTriggers(ids) => self.activate(&ids, &mut out),
            BackgroundCommand::DeactivateTriggers(ids) => self.deactivate(&ids, &mut out),
            BackgroundCommand::Restart => self.restart(&mut out),
            BackgroundCommand::Cancel(id) => self.cancel(id, &mut out),
            BackgroundCommand::CancelAll => self.cancel_all(&mut out),
            BackgroundCommand::ExpireDue => self.expire_due(&mut out),
            BackgroundCommand::Undo | BackgroundCommand::Redo => {
                let undo = matches!(command, BackgroundCommand::Undo);
                let result = if undo {
                    self.state.undo()
                } else {
                    self.state.redo()
                };
                if result.restored {
                    self.cancel_all(&mut out);
                    self.restart(&mut out);
                    self.persist(&mut out);
                    out.effects.push(BackgroundEffect::Sound {
                        event: if undo { "undo" } else { "redo" },
                    });
                }
                out.history = Some(result);
            }
            BackgroundCommand::Delete(id) => {
                // Workflow ignores missing IDs (unlike the raw mutation helper).
                if let Some(path) = find_path(self.state.tasks(), id) {
                    let result = self.state.delete_task(id);
                    for id in result.removed_task_ids {
                        self.cancel(id, &mut out);
                    }
                    // Removing an earlier sibling shifts surviving closures' paths.
                    for pending in &mut self.timers {
                        if let Some(target) = &mut pending.path {
                            let depth = path.len() - 1;
                            if target.len() > depth
                                && target[..depth] == path[..depth]
                                && target[depth] > path[depth]
                            {
                                target[depth] -= 1;
                            }
                        }
                    }
                    if result.removed {
                        self.persist(&mut out);
                    }
                }
            }
            BackgroundCommand::Clear => {
                if self.state.progress().total > 0 {
                    self.cancel_all(&mut out);
                    self.state.clear_tasks();
                    self.persist(&mut out);
                }
            }
            BackgroundCommand::Replace {
                tasks,
                current_template,
            } => {
                // TS timers capture references, not a future ID lookup. Retain
                // old targets so their callback can still save/log, but never
                // expire a new task that happens to reuse the ID or tree path.
                for pending in &mut self.timers {
                    if let Some(path) = pending.path.take() {
                        pending.detached = at_path(self.state.tasks(), &path).clone();
                    }
                }
                self.state.replace_tasks(tasks, current_template);
            }
            BackgroundCommand::Edit { id, text, priority } => {
                if self.state.edit_task(id, &text, priority) {
                    self.persist(&mut out);
                }
            }
            BackgroundCommand::UpdateTriggers { id, triggers } => {
                if self.state.update_triggers(id, triggers) {
                    self.persist(&mut out);
                }
            }
            BackgroundCommand::AddBackground { text, options } => {
                self.state.add_background_task(text, &options);
            }
            BackgroundCommand::AddTask { text, parent } => {
                if self.state.add_task(text, parent).is_some() {
                    self.persist(&mut out);
                }
            }
            BackgroundCommand::Move {
                id,
                target,
                above,
                subtask,
            } => {
                // Replay the SAME mutation over a labelled topology. IDs cannot
                // identify captured objects (collisions are supported), and value
                // equality cannot distinguish identical siblings. Text is never
                // consulted for move targeting, so labels preserve exact identity
                // without adding persisted fields or duplicating the move algorithm.
                let before = paths(self.state.tasks());
                let mut topology = self.state.tasks().to_vec();
                for (token, path) in before.iter().enumerate() {
                    at_path_mut(&mut topology, path).text = token.to_string();
                }
                for pending in &mut self.timers {
                    if let Some(path) = &pending.path {
                        pending.detached = at_path(self.state.tasks(), path).clone();
                    }
                }
                let result = self.state.move_task(id, target, above, subtask);
                if result.success {
                    let replay = move_task_in_tree(&mut topology, id, target, above, subtask);
                    debug_assert!(replay.success);
                    let after = paths(&topology);
                    for pending in &mut self.timers {
                        if let Some(old) = &pending.path {
                            let token =
                                before.iter().position(|p| p == old).expect("captured path");
                            let label = token.to_string();
                            pending.path = after
                                .iter()
                                .find(|p| at_path(&topology, p).text == label)
                                .cloned();
                            // The legacy self-subtask quirk loses the object. The
                            // captured closure remains detached, just as on Replace.
                        }
                    }
                    self.persist(&mut out);
                }
                // Failure restores the exact original array, so every captured
                // path remains valid without replay or changing its deadline.
                out.moved = Some(result);
            }
            BackgroundCommand::CaptureUndo(label) => self.state.capture_undo_state(&label),
            BackgroundCommand::Save => self.persist(&mut out),
        }
        out
    }

    fn persist(&mut self, out: &mut BackgroundTransition) {
        out.effects.push(BackgroundEffect::Persist {
            data: self.state.record_auto_save_snapshot(),
        });
    }
    fn activate(&mut self, ids: &[TaskId], out: &mut BackgroundTransition) {
        let mut count = 0;
        for &id in ids {
            let Some(path) = find_path(self.state.tasks(), id) else {
                continue;
            };
            let now = self.state.environment.now_iso();
            let task = at_path_mut(self.state.tasks_mut(), &path);
            if task.mode != TaskMode::Background || task.activated {
                continue;
            }
            // Completed but dormant tasks are activated too; the callback later
            // leaves them active, because it checks !completed at fire time.
            task.activated = true;
            task.activated_at = Some(now);
            count += 1;
            self.start(path, out);
        }
        if count > 0 {
            out.activated_count += count;
            out.effects.push(BackgroundEffect::Activated {
                count,
                message: format!(
                    "{count} background task{} activated!",
                    if count > 1 { "s" } else { "" }
                ),
            });
            out.effects.push(BackgroundEffect::Sound {
                event: "backgroundActivated",
            });
        }
    }
    fn deactivate(&mut self, ids: &[TaskId], out: &mut BackgroundTransition) {
        for &id in ids {
            let Some(path) = find_path(self.state.tasks(), id) else {
                continue;
            };
            let task = at_path_mut(self.state.tasks_mut(), &path);
            if task.mode == TaskMode::Background && task.activated && !task.completed {
                task.activated = false;
                task.activated_at = None;
                self.cancel(id, out);
            }
        }
    }
    fn restart(&mut self, out: &mut BackgroundTransition) {
        let paths = paths(self.state.tasks());
        for path in paths {
            let task = at_path(self.state.tasks(), &path);
            if task.mode == TaskMode::Background && task.activated && !task.completed {
                self.start(path, out);
            }
        }
    }
    fn start(&mut self, path: Vec<usize>, out: &mut BackgroundTransition) {
        let task = at_path(self.state.tasks(), &path).clone();
        let Some(minutes) = task
            .background_options
            .as_ref()
            .and_then(|o| o.expires_after_minutes)
        else {
            return;
        };
        if minutes == 0.0 || minutes.is_nan() {
            return;
        }
        self.cancel(task.id, out);
        let now = self.state.environment.now_ms();
        let delay = expiration_delay_ms(&task, minutes, now);
        if delay <= 0.0 {
            at_path_mut(self.state.tasks_mut(), &path).activated = false;
            self.persist(out);
            return;
        }
        // setTimeout truncates fractional milliseconds; overflowing delays in
        // the source fake-timer/runtime contract become a 1ms timer.
        let delay = if delay > 2_147_483_647.0 {
            1.0
        } else {
            delay.trunc()
        };
        let due_ms = now + delay;
        out.effects.push(BackgroundEffect::Schedule {
            id: task.id,
            due_ms,
        });
        self.sequence += 1;
        self.timers.push(PendingTimer {
            timer: ExpirationTimer {
                id: task.id,
                due_ms,
            },
            path: Some(path),
            detached: task,
            sequence: self.sequence,
        });
    }
    fn cancel(&mut self, id: TaskId, out: &mut BackgroundTransition) {
        if let Some(index) = self.timers.iter().position(|t| t.timer.id == id) {
            self.timers.remove(index);
            out.effects.push(BackgroundEffect::Cancel { id });
        }
    }
    fn cancel_all(&mut self, out: &mut BackgroundTransition) {
        // Record<number,Timer> uses JS property order: array-index keys first,
        // ascending, then other IDs in insertion order (including f64 legacy IDs).
        let mut ids: Vec<_> = self.timers.iter().map(|t| t.timer.id).collect();
        ids.sort_by(|a, b| match (array_index(*a), array_index(*b)) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });
        for id in ids {
            self.cancel(id, out);
        }
    }
    fn expire_due(&mut self, out: &mut BackgroundTransition) {
        while let Some(index) = self
            .timers
            .iter()
            .enumerate()
            .filter(|(_, t)| t.timer.due_ms <= self.state.environment.now_ms())
            .min_by(|(_, a), (_, b)| {
                a.timer
                    .due_ms
                    .total_cmp(&b.timer.due_ms)
                    .then(a.sequence.cmp(&b.sequence))
            })
            .map(|(i, _)| i)
        {
            let pending = &mut self.timers[index];
            let task = match &pending.path {
                Some(path) => at_path_mut(self.state.tasks_mut(), path),
                None => &mut pending.detached,
            };
            let expired_text = if task.activated && !task.completed {
                task.activated = false;
                Some(task.text.clone())
            } else {
                None
            };
            let id = pending.timer.id;
            if let Some(text) = expired_text {
                self.persist(out);
                out.effects.push(BackgroundEffect::Log {
                    message: format!("Background task \"{text}\" expired"),
                });
            }
            self.cancel(id, out);
        }
    }
}

fn array_index(id: TaskId) -> Option<u32> {
    (id.0 >= 0.0 && id.0 < f64::from(u32::MAX) && id.0.fract() == 0.0).then_some(id.0 as u32)
}

/// Missing/empty/invalid activation timestamps restart a full duration. A valid
/// future timestamp extends it. Immediate expiry deliberately retains activatedAt.
pub fn expiration_delay_ms(task: &Task, minutes: f64, now_ms: f64) -> f64 {
    let activated_ms = task
        .activated_at
        .as_deref()
        .and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|d| d.timestamp_millis())
                .or_else(|| {
                    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .ok()
                        .and_then(|d| d.and_hms_opt(0, 0, 0))
                        .map(|d| d.and_utc().timestamp_millis())
                })
        })
        .map(|ms| ms as f64);
    minutes * 60.0 * 1000.0 - activated_ms.map_or(0.0, |activated| now_ms - activated)
}
fn paths(tasks: &[Task]) -> Vec<Vec<usize>> {
    let mut result = vec![];
    for (index, task) in tasks.iter().enumerate() {
        result.push(vec![index]);
        for mut path in paths(&task.children) {
            path.insert(0, index);
            result.push(path);
        }
    }
    result
}
fn find_path(tasks: &[Task], id: TaskId) -> Option<Vec<usize>> {
    paths(tasks)
        .into_iter()
        .find(|p| at_path(tasks, p).id == id)
}
fn at_path<'a>(mut tasks: &'a [Task], path: &[usize]) -> &'a Task {
    let (last, parents) = path.split_last().expect("nonempty task path");
    for &index in parents {
        tasks = &tasks[index].children;
    }
    &tasks[*last]
}
fn at_path_mut<'a>(mut tasks: &'a mut [Task], path: &[usize]) -> &'a mut Task {
    let (last, parents) = path.split_last().expect("nonempty task path");
    for &index in parents {
        tasks = &mut tasks[index].children;
    }
    &mut tasks[*last]
}
