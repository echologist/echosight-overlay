use serde_json::Value;
use std::{fs, path::PathBuf};

mod task_support;
use task_support::{execute, first_difference};

#[test]
fn typescript_task_fixtures() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/tasks");
    let mut inputs: Vec<_> = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.to_string_lossy().ends_with(".input.json"))
        .collect();
    inputs.sort();
    assert!(!inputs.is_empty(), "TS fixtures must exist");
    let mut failures = Vec::new();
    for path in &inputs {
        let expected_path = path
            .to_string_lossy()
            .replace(".input.json", ".expected.json");
        let input: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        let expected_bytes = fs::read_to_string(expected_path).unwrap();
        let expected: Value = serde_json::from_str(&expected_bytes).unwrap();
        let actual = execute(&input);
        if actual != expected || serde_json::to_string_pretty(&actual).unwrap() != expected_bytes {
            eprintln!(
                "{}: {}",
                path.display(),
                if actual != expected {
                    first_difference(&actual, &expected, "$")
                } else {
                    let bytes = serde_json::to_string_pretty(&actual).unwrap();
                    let index = bytes
                        .bytes()
                        .zip(expected_bytes.bytes())
                        .position(|(a, e)| a != e)
                        .unwrap_or(bytes.len().min(expected_bytes.len()));
                    format!("byte mismatch at {index}")
                }
            );
            failures.push(path.file_name().unwrap().to_string_lossy().to_string());
        }
    }
    assert!(
        failures.is_empty(),
        "{}/{} TS fixtures differ: {failures:?}",
        failures.len(),
        inputs.len()
    );
}
