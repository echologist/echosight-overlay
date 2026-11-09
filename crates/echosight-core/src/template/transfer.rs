use super::{
    model::{TaskTemplate, TemplateData, TemplateError, TemplateExportData},
    normalize::normalize_template_data,
    ops::{count_template_tasks, has_background_task},
};
use crate::{
    jscompat::{js_string_to_number, js_trim, utf16_len},
    storage::{ParseError, json_format::json_format, parse_json},
    task::model::TaskEnvironment,
};

pub const MAX_IMPORT_LENGTH: usize = 100_000;

pub fn parse_template_import(input: &str) -> Result<TemplateData, TemplateError> {
    parse_template_import_with_limit(input, MAX_IMPORT_LENGTH)
}

pub fn parse_template_import_with_limit(
    input: &str,
    max_length: usize,
) -> Result<TemplateData, TemplateError> {
    let input = js_trim(input);
    if input.is_empty() {
        return Err(TemplateError::Invalid("Template data is empty".into()));
    }
    if utf16_len(input) > max_length {
        return Err(TemplateError::Invalid("Template data too large".into()));
    }
    // destr recognizes these non-JSON primitive values, all rejected by the
    // template shape guard. Nested nonfinite values still fail as Unsupported.
    let invalid_shape = || TemplateError::Invalid("Invalid JSON template format".into());
    if matches!(
        input.to_ascii_lowercase().as_str(),
        "undefined" | "nan" | "infinity" | "-infinity"
    ) {
        return Err(invalid_shape());
    }
    let parsed = parse_json(input).map_err(|error| match error {
        ParseError::Invalid(message) => TemplateError::Invalid(message),
        // A valid scalar decimal can overflow JS Number (e.g. 1e400). It is
        // still a primitive rejected by the template shape guard. Only classify
        // after parsing: malformed numeric syntax must retain the destr error.
        ParseError::Unsupported(_) if js_string_to_number(input).is_infinite() => invalid_shape(),
        ParseError::Unsupported(message) => TemplateError::Unsupported(message),
    })?;
    if !parsed.is_object() || !parsed["name"].is_string() || !parsed["tasks"].is_array() {
        return Err(invalid_shape());
    }
    let data = normalize_template_data(&parsed)?;
    if data.tasks.is_empty() {
        return Err(TemplateError::Invalid(
            "Template must contain at least one valid task".into(),
        ));
    }
    Ok(data)
}

pub fn create_template_export_data(
    template: &TaskTemplate,
    env: &impl TaskEnvironment,
) -> TemplateExportData {
    TemplateExportData {
        name: template.name.clone(),
        tasks: template.tasks.clone(),
        version: if has_background_task(&template.tasks) {
            "2.0"
        } else {
            "1.0"
        }
        .into(),
        exported_at: env.now_iso(),
        description: format!("PoE Task Template: {}", template.name),
        task_count: count_template_tasks(&template.tasks),
    }
}

pub fn create_template_export_filename(name: &str) -> String {
    let mut slug = String::new();
    let mut separator = false;
    for ch in name.to_lowercase().chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            if separator && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(ch);
            separator = false;
        } else {
            separator = true;
        }
    }
    if slug.is_empty() {
        slug.push_str("template");
    }
    format!("poe2-template-{slug}.json")
}

/// Pure download payload. The native dialog/file writer belongs to the shell.
#[derive(Clone, Debug, serde::Serialize)]
pub struct TemplateExport {
    pub data: TemplateExportData,
    pub bytes: String,
    pub filename: String,
}

pub fn export_template(
    template: &TaskTemplate,
    env: &impl TaskEnvironment,
) -> Result<TemplateExport, serde_json::Error> {
    let data = create_template_export_data(template, env);
    let bytes = json_format(&serde_json::to_value(&data)?)?;
    Ok(TemplateExport {
        data,
        bytes,
        filename: create_template_export_filename(&template.name),
    })
}
