use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Map, Value};

/// Legacy IDs are JavaScript numbers, not integers (and are compared exactly).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct TaskId(pub f64);

impl Serialize for TaskId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_number(&self.0, serializer)
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// JSON.stringify emits integral task IDs and counts without a `.0` suffix.
// This is deliberately limited to the finite, safely represented task domain.
pub(crate) fn serialize_number<S: Serializer>(n: &f64, s: S) -> Result<S::Ok, S::Error> {
    if n.fract() == 0.0 && n.abs() <= 9_007_199_254_740_991.0 {
        s.serialize_i64(*n as i64)
    } else {
        s.serialize_f64(*n)
    }
}

fn serialize_optional_number<S: Serializer>(n: &Option<f64>, s: S) -> Result<S::Ok, S::Error> {
    match n {
        Some(n) => serialize_number(n, s),
        None => s.serialize_none(),
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskMode {
    #[default]
    Main,
    Background,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundPriority {
    #[default]
    Normal,
    High,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundOptions {
    #[serde(serialize_with = "serialize_optional_number")]
    pub expires_after_minutes: Option<f64>,
    pub priority: BackgroundPriority,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: TaskId,
    pub text: String,
    pub completed: bool,
    pub created_at: String,
    #[serde(default)]
    pub children: Vec<Task>,
    #[serde(default)]
    pub mode: TaskMode,
    #[serde(default)]
    pub triggers: Vec<TaskId>,
    #[serde(default = "default_activated")]
    pub activated: bool,
    #[serde(default)]
    pub activated_at: Option<String>,
    #[serde(default)]
    pub background_options: Option<BackgroundOptions>,
    /// Preserve extensions when manipulating otherwise typed task records.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

fn default_activated() -> bool {
    true
}

/// Clock/entropy are injected; native adapters own wall-clock formatting.
/// `now_iso` must describe `now_ms` as an ISO UTC timestamp with milliseconds.
pub trait TaskEnvironment {
    fn now_ms(&self) -> f64;
    fn now_iso(&self) -> String;
    /// Same range as Math.random(): [0, 1).
    fn random(&mut self) -> f64;
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskStateSnapshot {
    pub tasks: Vec<Task>,
    pub current_template: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskSaveData {
    pub tasks: Vec<Task>,
    pub current_template: Option<String>,
    // The TS loader caps the array but does not validate legacy snapshot entries.
    pub snapshots: Vec<Value>,
}
