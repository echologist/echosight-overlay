use super::model::{TaskTemplate, TemplateData, TemplateTask};
use crate::jscompat::{js_string_to_number, js_trim};
use crate::task::{
    creation::{create_task_id, sanitize_background_options},
    model::{BackgroundOptions, BackgroundPriority, Task, TaskEnvironment, TaskId, TaskMode},
};
use serde_json::Map;

pub fn parse_template_id(value: &str) -> Option<TaskId> {
    if js_trim(value).is_empty() {
        return None;
    }
    let id = js_string_to_number(value);
    id.is_finite().then_some(TaskId(id))
}

pub fn find_template_by_id(templates: &[TaskTemplate], id: TaskId) -> Option<&TaskTemplate> {
    templates.iter().find(|template| template.id == id)
}

pub fn find_template_by_name<'a>(
    templates: &'a [TaskTemplate],
    name: &str,
) -> Option<&'a TaskTemplate> {
    templates.iter().find(|template| template.name == name)
}

pub fn upsert_template_by_name(
    templates: &[TaskTemplate],
    template: TaskTemplate,
) -> Vec<TaskTemplate> {
    let mut next: Vec<_> = templates
        .iter()
        .filter(|existing| existing.name != template.name)
        .cloned()
        .collect();
    next.push(template);
    next
}

pub fn remove_template_by_id(templates: &[TaskTemplate], id: TaskId) -> Vec<TaskTemplate> {
    templates
        .iter()
        .filter(|template| template.id != id)
        .cloned()
        .collect()
}

pub fn create_template_from_tasks(
    name: String,
    tasks: &[Task],
    env: &impl TaskEnvironment,
) -> TaskTemplate {
    TaskTemplate {
        id: TaskId(env.now_ms()),
        name,
        tasks: create_template_tasks(tasks),
        created_at: env.now_iso(),
        imported: false,
        community: false,
    }
}

pub fn create_imported_template(data: TemplateData, env: &impl TaskEnvironment) -> TaskTemplate {
    TaskTemplate {
        id: TaskId(env.now_ms()),
        name: data.name,
        tasks: data.tasks,
        created_at: env.now_iso(),
        imported: true,
        community: false,
    }
}

/// Incomplete preorder only. Completed parents hide their entire subtree.
/// Equal source IDs intentionally resolve to their LAST incomplete occurrence.
pub fn create_template_tasks(tasks: &[Task]) -> Vec<TemplateTask> {
    fn index(tasks: &[Task], ids: &mut Vec<TaskId>) {
        for task in tasks.iter().filter(|t| !t.completed) {
            ids.push(task.id);
            index(&task.children, ids);
        }
    }
    fn strip(tasks: &[Task], ids: &[TaskId]) -> Vec<TemplateTask> {
        tasks
            .iter()
            .filter(|t| !t.completed)
            .map(|task| {
                let children = strip(&task.children, ids);
                let trigger_indices: Vec<_> = task
                    .triggers
                    .iter()
                    .filter_map(|id| ids.iter().rposition(|other| other == id).map(|i| i as f64))
                    .collect();
                TemplateTask {
                    text: task.text.clone(),
                    children: (!children.is_empty()).then_some(children),
                    mode: (task.mode == TaskMode::Background).then_some(TaskMode::Background),
                    background_options: (task.mode == TaskMode::Background)
                        .then(|| task.background_options.clone()),
                    trigger_indices: (!trigger_indices.is_empty()).then_some(trigger_indices),
                    ..Default::default()
                }
            })
            .collect()
    }
    let mut ids = Vec::new();
    index(tasks, &mut ids);
    strip(tasks, &ids)
}

pub fn create_runtime_task_from_template(
    template: &TemplateTask,
    inherited_mode: TaskMode,
    env: &mut impl TaskEnvironment,
) -> Task {
    let mode =
        if template.mode == Some(TaskMode::Background) || inherited_mode == TaskMode::Background {
            TaskMode::Background
        } else {
            TaskMode::Main
        };
    Task {
        id: create_task_id(env),
        text: template.text.clone(),
        completed: false,
        created_at: env.now_iso(),
        children: template
            .children
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|child| create_runtime_task_from_template(child, mode, env))
            .collect(),
        mode,
        triggers: vec![],
        activated: mode != TaskMode::Background,
        activated_at: None,
        // Re-sanitize: a normalized fractional duration can be zero; runtime
        // creation sanitizes that zero again to null, exactly like TS.
        background_options: (mode == TaskMode::Background).then(|| {
            template
                .background_options
                .as_ref()
                .and_then(Option::as_ref)
                .and_then(|options| {
                    sanitize_background_options(
                        &serde_json::to_value(options).expect("background options serialize"),
                    )
                })
                .unwrap_or(BackgroundOptions {
                    expires_after_minutes: None,
                    priority: BackgroundPriority::Normal,
                })
        }),
        extra: Map::new(),
    }
}

pub fn instantiate_template_tasks(
    template: &TaskTemplate,
    env: &mut impl TaskEnvironment,
) -> Vec<Task> {
    let mut tasks: Vec<_> = template
        .tasks
        .iter()
        .map(|t| create_runtime_task_from_template(t, TaskMode::Main, env))
        .collect();
    fn collect(tasks: &[Task], ids: &mut Vec<TaskId>) {
        for task in tasks {
            ids.push(task.id);
            collect(&task.children, ids);
        }
    }
    fn remap(templates: &[TemplateTask], tasks: &mut [Task], ids: &[TaskId]) {
        for (template, task) in templates.iter().zip(tasks) {
            if let Some(indices) = &template.trigger_indices {
                task.triggers = indices
                    .iter()
                    .filter(|&&index| {
                        index.is_finite()
                            && index >= 0.0
                            && index.fract() == 0.0
                            && index < ids.len() as f64
                    })
                    .map(|&index| ids[index as usize])
                    .collect();
            }
            remap(
                template.children.as_deref().unwrap_or_default(),
                &mut task.children,
                ids,
            );
        }
    }
    let mut ids = Vec::new();
    collect(&tasks, &mut ids);
    // Walk structural positions, never look up by ID: generated IDs can collide.
    remap(&template.tasks, &mut tasks, &ids);
    tasks
}

pub fn count_template_tasks(tasks: &[TemplateTask]) -> usize {
    tasks
        .iter()
        .map(|t| 1 + count_template_tasks(t.children.as_deref().unwrap_or_default()))
        .sum()
}

pub fn has_background_task(tasks: &[TemplateTask]) -> bool {
    tasks.iter().any(|t| {
        t.mode == Some(TaskMode::Background)
            || has_background_task(t.children.as_deref().unwrap_or_default())
    })
}

pub fn has_template_trigger_indices(tasks: &[TemplateTask]) -> bool {
    tasks.iter().any(|t| {
        t.trigger_indices.as_ref().is_some_and(|i| !i.is_empty())
            || has_template_trigger_indices(t.children.as_deref().unwrap_or_default())
    })
}

/// Import/community callers ask this BEFORE creating a new template (so cancel
/// consumes no clock/entropy). Save Template intentionally never asks it.
pub fn replacement_prompt(templates: &[TaskTemplate], name: &str) -> Option<String> {
    find_template_by_name(templates, name)
        .map(|_| format!("Template \"{name}\" already exists. Replace it?"))
}
