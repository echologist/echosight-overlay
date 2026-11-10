//! Standalone subprocess entry point; never put fatal allocation probes in the test host.
use echosight_core::theme::v2::{Severity, Theme};
use serde_json::{Value, json};

#[test]
#[ignore = "run by theme_v2_budget_check.py under process resource limits"]
fn budget_probe() {
    let path = std::env::var("THEME_V2_BUDGET_INPUT").expect("input file");
    let input: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let result = match Theme::parse(&input) {
        Ok(theme) => json!({"ok":true,"diagnostics":theme.diagnostics().len()}),
        Err(d) => {
            json!({"ok":false,"diagnostics":d.len(),"budget":d.iter().any(|d|d.code == "budget" && d.severity == Severity::Error),"budget_messages":d.iter().filter(|d|d.code == "budget").map(|d|d.message.as_str()).collect::<Vec<_>>()})
        }
    };
    println!("BUDGET_RESULT={result}");
}
