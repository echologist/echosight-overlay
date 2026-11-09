use echosight_core::storage::{ParseError, json_format, parse_json};
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!("golden/storage/storage.json")).unwrap()
}

#[test]
fn parser_matches_installed_destr_for_the_value_compatible_slice() {
    for case in fixture()["parse"].as_array().unwrap() {
        let input = case["input"].as_str().unwrap();
        match case["status"].as_str().unwrap() {
            "ok" => assert_eq!(
                json_format(&parse_json(input).unwrap()).unwrap(),
                case["pretty"],
                "{input:?}"
            ),
            "invalid" => assert!(
                matches!(parse_json(input), Err(ParseError::Invalid(_))),
                "{input:?}: {:?}",
                parse_json(input)
            ),
            "unsupported" => assert!(
                matches!(parse_json(input), Err(ParseError::Unsupported(_))),
                "{input:?}: {:?}",
                parse_json(input)
            ),
            other => panic!("unexpected fixture status {other}"),
        }
    }
}

#[test]
fn pretty_bytes_match_real_ts_writer() {
    for case in fixture()["writes"].as_array().unwrap() {
        assert_eq!(json_format(&case["value"]).unwrap(), case["bytes"]);
    }
}

#[test]
fn excessive_nesting_is_not_mistaken_for_corruption() {
    let text = format!("{}0{}", "[".repeat(150), "]".repeat(150));
    assert!(matches!(parse_json(&text), Err(ParseError::Unsupported(_))));
}
