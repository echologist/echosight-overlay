//! Pure adapters for the templatePersistence.ts result contract. Native I/O
//! supplies its Result; diagnostics are returned, never logged behind the UI.
use super::{
    model::{TaskTemplate, TemplateError},
    normalize::normalize_template_collection,
};
use crate::task::model::TaskEnvironment;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct PersistenceDiagnostic {
    pub prefix: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PersistenceOutcome<T> {
    pub result: T,
    pub diagnostic: Option<PersistenceDiagnostic>,
}

pub fn resolve_template_load(
    loaded: Result<&Value, &str>,
    env: &mut impl TaskEnvironment,
) -> Result<PersistenceOutcome<Vec<TaskTemplate>>, TemplateError> {
    match loaded {
        Ok(value) => Ok(PersistenceOutcome {
            result: normalize_template_collection(value, env)?,
            diagnostic: None,
        }),
        Err(message) => Ok(PersistenceOutcome {
            result: vec![],
            diagnostic: Some(PersistenceDiagnostic {
                prefix: "Failed to load templates:",
                message: message.into(),
            }),
        }),
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct TemplateSaveResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Only publish the proposed collection to application state when result=true.
/// A transport rejection and a success:false response both leave it unpublished.
pub fn resolve_template_save(
    saved: Result<TemplateSaveResponse, &str>,
) -> PersistenceOutcome<bool> {
    let message = match saved {
        Ok(response) if response.success => {
            return PersistenceOutcome {
                result: true,
                diagnostic: None,
            };
        }
        Ok(response) => response
            .error
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Unknown save error".into()),
        Err(message) => message.into(),
    };
    PersistenceOutcome {
        result: false,
        diagnostic: Some(PersistenceDiagnostic {
            prefix: "Failed to save templates:",
            message,
        }),
    }
}
