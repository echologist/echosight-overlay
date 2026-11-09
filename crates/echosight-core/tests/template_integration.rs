use echosight_core::{
    storage::{DataPaths, FileKind, Storage, json_format},
    task::model::{TaskEnvironment, TaskMode},
    template::{community::*, model::*, normalize::*, ops::*, persistence::*, transfer::*},
};
use serde_json::{Value, json};

struct Env {
    ms: f64,
}
impl TaskEnvironment for Env {
    fn now_ms(&self) -> f64 {
        self.ms
    }
    fn now_iso(&self) -> String {
        "2025-06-14T22:09:14.072Z".into()
    }
    fn random(&mut self) -> f64 {
        0.5
    }
}

#[test]
fn unsupported_js_values_fail_closed_without_lossy_replacement() {
    let fixture: Value = serde_json::from_str(include_str!("golden/templates/cases.json")).unwrap();
    for case in fixture["unsupported"].as_array().unwrap() {
        assert!(
            case["outputJson"].is_string(),
            "real TS accepted this input"
        );
        assert!(
            matches!(
                parse_template_import(case["input"].as_str().unwrap()),
                Err(TemplateError::Unsupported(_))
            ),
            "{case}"
        );
    }
}

#[test]
fn numeric_load_rejections_never_produce_a_persistable_collection() {
    let fixture: Value = serde_json::from_str(include_str!("golden/templates/cases.json")).unwrap();
    let mut env = Env {
        ms: 1749938954072.0,
    };
    for case in fixture["loadRejections"].as_array().unwrap() {
        // Actual TS fallback/diagnostics, not a fabricated normalized document.
        assert_eq!(case["result"], json!([]));
        assert_eq!(case["logs"][0][0], "Failed to load templates:");
        assert_eq!(case["logs"][0][1], "TypeError");
        match resolve_template_load(Ok(&case["input"]), &mut env) {
            Err(TemplateError::NumericCoercion(error)) => {
                assert_eq!(case["logs"][0][2], error.to_string());
            }
            other => panic!("load must fail without a normalized collection: {other:?}"),
        }
    }
}

#[test]
fn syntax_errors_are_rejected_without_fabricating_v8_diagnostics() {
    let fixture: Value = serde_json::from_str(include_str!("golden/templates/cases.json")).unwrap();
    for case in fixture["syntaxErrors"].as_array().unwrap() {
        assert!(!case["message"].as_str().unwrap().is_empty());
        assert!(matches!(
            parse_template_import(case["input"].as_str().unwrap()),
            Err(TemplateError::Invalid(_))
        ));
    }
}

#[test]
fn native_storage_roundtrip_normalizes_only_on_load() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::new(dir.path()).unwrap();
    let storage = Storage::new(paths.clone());
    let mut env = Env {
        ms: 1749938954072.0,
    };
    let original = create_community_template(
        &COMMUNITY_TEMPLATES[0],
        create_community_template_tasks(&COMMUNITY_TEMPLATES[0]),
        &env,
    );
    let collection = vec![original];
    let raw = serde_json::to_value(&collection).unwrap();
    storage.save(FileKind::Templates, &raw).unwrap();
    assert_eq!(
        std::fs::read_to_string(paths.file(FileKind::Templates)).unwrap(),
        json_format(&raw).unwrap()
    );
    let reloaded = storage.load(FileKind::Templates).unwrap();
    assert_eq!(reloaded, raw);
    let result = resolve_template_load(Ok(&reloaded), &mut env).unwrap();
    assert!(result.diagnostic.is_none());
    assert_eq!(
        result.result[0].tasks.len(),
        COMMUNITY_TEMPLATES[0].tasks.len()
    );
    // Empty children survive community creation/export, but normalization omits them.
    assert!(collection[0].tasks[0].children.is_some());
    assert!(result.result[0].tasks[0].children.is_none());
}

#[test]
fn loading_twice_uses_new_ids_and_never_mutates_template() {
    let mut env = Env {
        ms: 1749938954072.0,
    };
    let input = json!({"name":"nested","tasks":[{"text":"root","children":[{"text":"child","children":[{"text":"leaf","triggerIndices":[0,2]}]}]}, {"text":"bg","mode":"background","triggerIndices":[2]}]});
    let data = normalize_template_data(&input).unwrap();
    let template = create_imported_template(data, &env);
    let before = serde_json::to_value(&template).unwrap();
    let first = instantiate_template_tasks(&template, &mut env);
    env.ms += 1.0;
    let second = instantiate_template_tasks(&template, &mut env);
    assert_ne!(first[0].id, second[0].id);
    assert_eq!(
        second[0].children[0].children[0].triggers,
        vec![second[0].id, second[0].children[0].children[0].id]
    );
    assert_eq!(
        second[1].triggers,
        vec![second[0].children[0].children[0].id]
    );
    assert_eq!(second[1].mode, TaskMode::Background);
    assert!(!second[1].activated);
    assert_eq!(serde_json::to_value(&template).unwrap(), before);
}

#[test]
fn all_completed_tasks_save_an_empty_template_that_reload_drops() {
    let mut env = Env {
        ms: 1749938954072.0,
    };
    let mut task = create_runtime_task_from_template(
        &TemplateTask {
            text: "done".into(),
            ..Default::default()
        },
        TaskMode::Main,
        &mut env,
    );
    task.completed = true;
    let template = create_template_from_tasks("empty".into(), &[task], &env);
    assert!(template.tasks.is_empty());
    assert!(
        normalize_saved_template(&serde_json::to_value(template).unwrap(), &mut env)
            .unwrap()
            .is_none()
    );
}

#[test]
fn replacement_prompt_is_separate_from_silent_save_and_cancel() {
    let env = Env {
        ms: 1749938954072.0,
    };
    let data = TemplateData {
        name: "same".into(),
        tasks: vec![],
    };
    let existing = vec![create_imported_template(data.clone(), &env)];
    let before = serde_json::to_value(&existing).unwrap();
    assert_eq!(
        replacement_prompt(&existing, "same").as_deref(),
        Some("Template \"same\" already exists. Replace it?")
    );
    assert!(replacement_prompt(&existing, "Same").is_none());
    assert_eq!(serde_json::to_value(&existing).unwrap(), before);
    let next = upsert_template_by_name(&existing, create_imported_template(data, &env));
    assert_eq!(next.len(), 1);
}

#[test]
fn import_limit_counts_trimmed_utf16_code_units_not_utf8_bytes() {
    let text = "{\"name\":\"x\",\"tasks\":[\"😀\"]}";
    let units = text.encode_utf16().count();
    assert!(parse_template_import_with_limit(&format!("\u{feff}{text}  "), units).is_ok());
    assert!(
        matches!(parse_template_import_with_limit(text, units - 1), Err(TemplateError::Invalid(message)) if message == "Template data too large")
    );
}
