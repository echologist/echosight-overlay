use crate::task::model::{BackgroundOptions, TaskId, TaskMode};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A template node, not a runtime task. Optional fields retain absence versus
/// explicit empty children/null options because exported JSON preserves those.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TemplateTask {
    pub text: String,
    pub children: Option<Vec<TemplateTask>>,
    pub mode: Option<TaskMode>,
    pub background_options: Option<Option<BackgroundOptions>>,
    pub trigger_indices: Option<Vec<f64>>,
    /// Raw typed callers can carry extensions; normalization deliberately strips them.
    pub extra: serde_json::Map<String, serde_json::Value>,
    /// Original JSON property order, retained for exact export bytes.
    pub field_order: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TemplateTaskRecord {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TemplateTask>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<TaskMode>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_options"
    )]
    pub background_options: Option<Option<BackgroundOptions>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "indices"
    )]
    pub trigger_indices: Option<Vec<f64>>,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

impl<'de> Deserialize<'de> for TemplateTask {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(d)?;
        let field_order = value
            .as_object()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        let record: TemplateTaskRecord =
            serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        Ok(Self {
            text: record.text,
            children: record.children,
            mode: record.mode,
            background_options: record.background_options,
            trigger_indices: record.trigger_indices,
            extra: record.extra,
            field_order,
        })
    }
}

impl Serialize for TemplateTask {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let record = TemplateTaskRecord {
            text: self.text.clone(),
            children: self.children.clone(),
            mode: self.mode,
            background_options: self.background_options.clone(),
            trigger_indices: self.trigger_indices.clone(),
            extra: self.extra.clone(),
        };
        let serde_json::Value::Object(mut fields) =
            serde_json::to_value(record).map_err(serde::ser::Error::custom)?
        else {
            unreachable!()
        };
        let mut ordered = serde_json::Map::new();
        for key in &self.field_order {
            if let Some(value) = fields.shift_remove(key) {
                ordered.insert(key.clone(), value);
            }
        }
        ordered.extend(fields);
        ordered.serialize(s)
    }
}

fn present_options<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<BackgroundOptions>>, D::Error> {
    Option::<BackgroundOptions>::deserialize(deserializer).map(Some)
}

fn indices<S: Serializer>(values: &Option<Vec<f64>>, s: S) -> Result<S::Ok, S::Error> {
    values
        .as_ref()
        .map(|v| v.iter().copied().map(TaskId).collect::<Vec<_>>())
        .serialize(s)
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct TemplateData {
    pub name: String,
    pub tasks: Vec<TemplateTask>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskTemplate {
    pub id: TaskId,
    pub name: String,
    pub tasks: Vec<TemplateTask>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub imported: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub community: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TemplateExportData {
    pub name: String,
    pub tasks: Vec<TemplateTask>,
    pub version: String,
    pub exported_at: String,
    pub description: String,
    pub task_count: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error(transparent)]
    NumericCoercion(#[from] crate::jscompat::JsNumberError),
    #[error("{0}")]
    Invalid(String),
    /// Valid JS values that cannot be carried by Rust strings/serde_json. Do
    /// not silently truncate, replace with U+FFFD, or quarantine as corrupt.
    #[error("Template outside the Value compatibility slice: {0}")]
    Unsupported(String),
}
