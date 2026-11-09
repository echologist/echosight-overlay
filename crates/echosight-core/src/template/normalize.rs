use super::model::{TaskTemplate, TemplateData, TemplateError, TemplateTask};
use crate::jscompat::{
    checked_js_number, js_is_integer, js_trim, js_truthy, utf16_len, utf16_prefix,
};
use crate::task::{
    creation::{create_task_id, sanitize_background_options},
    model::{TaskEnvironment, TaskId, TaskMode},
};
use serde_json::Value;

pub fn normalize_template_text(value: &Value, max_length: usize) -> Result<String, TemplateError> {
    let Some(text) = value.as_str() else {
        return Ok(String::new());
    };
    let text = js_trim(text);
    let prefix = utf16_prefix(text, max_length);
    if utf16_len(&prefix) < max_length && utf16_len(text) > max_length {
        return Err(TemplateError::Unsupported(
            "UTF-16 truncation would split a surrogate pair".into(),
        ));
    }
    Ok(prefix)
}

pub fn normalize_template_task(value: &Value) -> Result<Option<TemplateTask>, TemplateError> {
    let text = normalize_template_text(
        if value.is_string() {
            value
        } else {
            &value["text"]
        },
        200,
    )?;
    if text.is_empty() {
        return Ok(None);
    }
    let children = normalize_tasks(&value["children"])?;
    let background_options = value
        .get("backgroundOptions")
        .filter(|v| js_truthy(v))
        .map(|options| {
            // The shared task sanitizer is infallible only for non-throwing
            // Number inputs. Check exactly the field it coerces, not unused data.
            if let Some(minutes) = options
                .as_object()
                .and_then(|o| o.get("expiresAfterMinutes"))
            {
                checked_js_number(minutes)?;
            }
            Ok::<_, TemplateError>(sanitize_background_options(options))
        })
        .transpose()?;
    let mut trigger_indices = Vec::new();
    if let Some(values) = value["triggerIndices"].as_array() {
        for value in values {
            let index = checked_js_number(value)?;
            if js_is_integer(index) && index >= 0.0 && !trigger_indices.contains(&index) {
                trigger_indices.push(index);
            }
        }
    }
    Ok(Some(TemplateTask {
        text,
        children: (!children.is_empty()).then_some(children),
        mode: (value["mode"].as_str() == Some("background")).then_some(TaskMode::Background),
        background_options,
        trigger_indices: (!trigger_indices.is_empty()).then_some(trigger_indices),
        ..Default::default()
    }))
}

fn normalize_tasks(value: &Value) -> Result<Vec<TemplateTask>, TemplateError> {
    let Some(tasks) = value.as_array() else {
        return Ok(vec![]);
    };
    tasks
        .iter()
        .filter_map(|task| normalize_template_task(task).transpose())
        .collect()
}

pub fn normalize_template_data(value: &Value) -> Result<TemplateData, TemplateError> {
    let name = normalize_template_text(&value["name"], 50)?;
    Ok(TemplateData {
        name: if name.is_empty() {
            "Imported Template".into()
        } else {
            name
        },
        tasks: normalize_tasks(&value["tasks"])?,
    })
}

pub fn normalize_saved_template(
    value: &Value,
    env: &mut impl TaskEnvironment,
) -> Result<Option<TaskTemplate>, TemplateError> {
    if !value.is_object() {
        return Ok(None);
    }
    let data = normalize_template_data(value)?;
    if data.tasks.is_empty() {
        return Ok(None);
    }
    let id = value
        .get("id")
        .map(checked_js_number)
        .transpose()?
        .unwrap_or(f64::NAN);
    // Source obtains the fallback timestamp before obtaining a fallback ID.
    let created_at = value["createdAt"]
        .as_str()
        .filter(|s| !js_trim(s).is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| env.now_iso());
    Ok(Some(TaskTemplate {
        id: if id.is_finite() {
            TaskId(id)
        } else {
            create_task_id(env)
        },
        name: data.name,
        tasks: data.tasks,
        created_at,
        imported: value["imported"] == true,
        community: value["community"] == true,
    }))
}

pub fn normalize_template_collection(
    value: &Value,
    env: &mut impl TaskEnvironment,
) -> Result<Vec<TaskTemplate>, TemplateError> {
    let Some(templates) = value.as_array() else {
        return Ok(vec![]);
    };
    templates
        .iter()
        .filter_map(|template| normalize_saved_template(template, env).transpose())
        .collect()
}
