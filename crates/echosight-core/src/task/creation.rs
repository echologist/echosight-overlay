use super::model::{
    BackgroundOptions, BackgroundPriority, Task, TaskEnvironment, TaskId, TaskMode,
};
use crate::jscompat::js_number;
use serde_json::{Map, Value};

pub fn create_task_id(env: &mut impl TaskEnvironment) -> TaskId {
    TaskId(env.now_ms() * 1000.0 + (env.random() * 1000.0).floor())
}

pub fn sanitize_background_options(options: &Value) -> Option<BackgroundOptions> {
    let options = options.as_object()?;
    // Missing property is undefined -> NaN, whereas explicit null -> zero.
    let minutes = options
        .get("expiresAfterMinutes")
        .map(js_number)
        .unwrap_or(f64::NAN);
    Some(BackgroundOptions {
        expires_after_minutes: (minutes.is_finite() && minutes > 0.0).then(|| minutes.floor()),
        priority: if options.get("priority").and_then(Value::as_str) == Some("high") {
            BackgroundPriority::High
        } else {
            BackgroundPriority::Normal
        },
    })
}

pub fn create_task(text: String, parent: Option<&Task>, env: &mut impl TaskEnvironment) -> Task {
    let mode = parent.map(|p| p.mode).unwrap_or_default();
    Task {
        id: create_task_id(env),
        text,
        completed: false,
        created_at: env.now_iso(),
        children: vec![],
        mode,
        triggers: vec![],
        activated: mode != TaskMode::Background || parent.is_some_and(|p| p.activated),
        activated_at: None,
        background_options: None,
        extra: Map::new(),
    }
}

pub fn create_background_task(
    text: String,
    options: &Value,
    env: &mut impl TaskEnvironment,
) -> Task {
    let mut task = create_task(text, None, env);
    task.mode = TaskMode::Background;
    task.activated = false;
    task.background_options = Some(sanitize_background_options(options).unwrap_or(
        BackgroundOptions {
            expires_after_minutes: None,
            priority: BackgroundPriority::Normal,
        },
    ));
    task
}
