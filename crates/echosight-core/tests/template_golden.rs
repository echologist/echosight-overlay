use serde_json::Value;

fn run_case(_case: &Value, _fixture: &Value) -> Value {
    // Compiling RED adapter: replaced with production calls after recording failure.
    Value::Null
}

fn check(operation: &str) {
    let fixture: Value = serde_json::from_str(include_str!("golden/templates/cases.json")).unwrap();
    let cases: Vec<_> = fixture["cases"].as_array().unwrap().iter()
        .filter(|case| case["op"].as_str() == Some(operation)).collect();
    assert!(!cases.is_empty(), "missing oracle for {operation}");
    for case in cases {
        assert_eq!(run_case(case, &fixture), case["expected"], "{}", case["name"]);
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
    parse_ids => "parseId", filenames => "filename", import_validation => "import",
    incomplete_preorder => "strip", create_templates => "create", instantiate => "instantiate",
    export_bytes => "export", recursive_inspection => "inspect", upsert => "upsert",
    remove => "remove", find_id => "findId", find_name => "findName",
    imported => "createImported", community_catalog => "community",
    persistence_load => "load", persistence_save => "save",
}
