use echosight_core::{
    task::model::{TaskEnvironment, TaskId},
    template::{community::*, model::*, normalize::*, ops::*, persistence::*, transfer::*},
};
use serde::Serialize;
use serde_json::{Value, json};

struct Env {
    ms: f64,
    iso: String,
    entropy: Vec<f64>,
    index: usize,
}
impl TaskEnvironment for Env {
    fn now_ms(&self) -> f64 {
        self.ms
    }
    fn now_iso(&self) -> String {
        self.iso.clone()
    }
    fn random(&mut self) -> f64 {
        let value = self.entropy[self.index % self.entropy.len()];
        self.index += 1;
        value
    }
}
fn value(value: impl Serialize) -> Value {
    serde_json::to_value(value).unwrap()
}
fn normalized<T: Serialize>(result: Result<T, TemplateError>) -> Value {
    match result {
        Ok(result) => value(result),
        Err(TemplateError::NumericCoercion(error)) => {
            json!({"thrown": {"name": "TypeError", "message": error.to_string()}})
        }
        Err(error) => panic!("unexpected template error: {error}"),
    }
}
fn outcome<T: Serialize>(outcome: PersistenceOutcome<T>) -> Value {
    let logs: Vec<_> = outcome
        .diagnostic
        .into_iter()
        .map(|d| vec![d.prefix.to_owned(), d.message])
        .collect();
    json!({"result": outcome.result, "logs": logs})
}
fn run_case(case: &Value, fixture: &Value) -> Value {
    let mut env = Env {
        ms: fixture["nowMs"].as_f64().unwrap(),
        iso: fixture["nowIso"].as_str().unwrap().into(),
        entropy: serde_json::from_value(case["randoms"].clone()).unwrap(),
        index: 0,
    };
    let input = &case["input"];
    match case["op"].as_str().unwrap() {
        "normalizeTask" => normalized(normalize_template_task(input)),
        "normalizeData" => normalized(normalize_template_data(input)),
        "normalizeSaved" => normalized(normalize_saved_template(input, &mut env)),
        "normalizeCollection" => normalized(normalize_template_collection(input, &mut env)),
        "number" => {
            let number = match echosight_core::jscompat::checked_js_number(input) {
                Ok(number) => number,
                Err(error) => return json!({"thrown": {"name": "TypeError", "message": error.to_string()}}),
            };
            if number.is_nan() { json!({"kind": "NaN"}) }
            else if number.is_infinite() { json!({"kind": if number > 0.0 { "Infinity" } else { "-Infinity" }}) }
            else { json!({"kind": "finite", "bits": format!("{:016x}", number.to_bits())}) }
        }
        "parseNumberId" => value(parse_template_id(input.as_str().unwrap()).map(|id| format!("{:016x}", id.0.to_bits()))),
        "parseId" => value(parse_template_id(input.as_str().unwrap())),
        "filename" => value(create_template_export_filename(input.as_str().unwrap())),
        "import" => {
            let text = input["text"].as_str().map(str::to_owned).unwrap_or_else(|| format!("{{\"name\":\"x\",\"tasks\":[\"{}\"]}}", input["suffix"].as_str().unwrap().repeat(input["pad"].as_u64().unwrap() as usize)));
            match parse_template_import(&text) { Ok(data) => json!({"ok": data}), Err(error) => json!({"error": error.to_string()}) }
        }
        "strip" => value(create_template_tasks(&serde_json::from_value::<Vec<_>>(input.clone()).unwrap())),
        "create" => value(create_template_from_tasks(input["name"].as_str().unwrap().into(), &serde_json::from_value::<Vec<_>>(input["tasks"].clone()).unwrap(), &env)),
        "instantiate" => value(instantiate_template_tasks(&serde_json::from_value(input.clone()).unwrap(), &mut env)),
        "export" => value(export_template(&serde_json::from_value(input.clone()).unwrap(), &env).unwrap()),
        "inspect" => {
            let tasks: Vec<TemplateTask> = serde_json::from_value(input.clone()).unwrap();
            json!({"count": count_template_tasks(&tasks), "background": has_background_task(&tasks), "triggers": has_template_trigger_indices(&tasks)})
        }
        "upsert" => value(upsert_template_by_name(&serde_json::from_value::<Vec<_>>(input["existing"].clone()).unwrap(), serde_json::from_value(input["replacement"].clone()).unwrap())),
        "remove" | "findId" | "findName" => {
            let existing: Vec<TaskTemplate> = serde_json::from_value(input["existing"].clone()).unwrap();
            match case["op"].as_str().unwrap() {
                "remove" => value(remove_template_by_id(&existing, TaskId(input["id"].as_f64().unwrap()))),
                "findId" => value(find_template_by_id(&existing, TaskId(input["id"].as_f64().unwrap()))),
                _ => value(find_template_by_name(&existing, input["name"].as_str().unwrap())),
            }
        }
        "createImported" => value(create_imported_template(serde_json::from_value(input.clone()).unwrap(), &env)),
        "community" => value(COMMUNITY_TEMPLATES.iter().map(|entry| {
            json!({"name": entry.name, "description": entry.description, "tasks": entry.tasks,
                "preview": create_community_template_preview(entry),
                "template": create_community_template(entry, create_community_template_tasks(entry), &env)})
        }).collect::<Vec<_>>()),
        "load" => outcome(resolve_template_load(input["failure"].as_str().map_or(Ok(&input["load"]), Err), &mut env).unwrap()),
        "save" => outcome(resolve_template_save(input["failure"].as_str().map_or_else(|| Ok(serde_json::from_value(input["response"].clone()).unwrap()), Err))),
        operation => panic!("unhandled oracle operation {operation}"),
    }
}

fn check(operation: &str) {
    let fixture: Value = serde_json::from_str(include_str!("golden/templates/cases.json")).unwrap();
    let cases: Vec<_> = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["op"].as_str() == Some(operation))
        .collect();
    assert!(!cases.is_empty(), "missing oracle for {operation}");
    for case in cases {
        assert_eq!(
            run_case(case, &fixture),
            case["expected"],
            "{}",
            case["name"]
        );
    }
}

macro_rules! oracle_tests {
    ($($name:ident => $operation:literal),* $(,)?) => { $(
        #[test] fn $name() { check($operation); }
    )* };
}
oracle_tests! {
    normalize_tasks => "normalizeTask", normalize_data => "normalizeData",
    normalize_saved => "normalizeSaved", normalize_collection => "normalizeCollection",
    numbers => "number", radix_ids => "parseNumberId", parse_ids => "parseId", filenames => "filename", import_validation => "import",
    incomplete_preorder => "strip", create_templates => "create", instantiate => "instantiate",
    export_bytes => "export", recursive_inspection => "inspect", upsert => "upsert",
    remove => "remove", find_id => "findId", find_name => "findName",
    imported => "createImported", community_catalog => "community",
    persistence_load => "load", persistence_save => "save",
}
