use echosight_core::theme::v2::{SCHEMA, State, Theme};
use serde_json::{Value, json};

#[test]
fn shared_schema_corpus_matches_runtime_acceptance() {
    let cases: Vec<Value> = serde_json::from_str(include_str!("theme_v2_cases.json")).unwrap();
    for case in cases {
        let result = Theme::parse(&case["document"]);
        assert_eq!(
            result.is_ok(),
            case["valid"].as_bool().unwrap(),
            "{}: {result:#?}",
            case["name"]
        );
        if let Ok(theme) = result {
            for name in theme.slot_names() {
                for states in [&[][..], &State::PRECEDENCE[..]] {
                    let style = theme
                        .slot(name, states, 0.5)
                        .expect("accepted slot must resolve");
                    if style.properties().get("fill").is_some() {
                        assert!(
                            style.fill().is_some(),
                            "{}: {name}: {:?}",
                            case["name"],
                            style.properties()
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn schema_declared_slots_states_and_properties_are_exercisable() {
    let schema: Value = serde_json::from_str(SCHEMA).unwrap();
    assert_eq!(
        schema["x-state-precedence"],
        json!(
            State::PRECEDENCE
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
        )
    );
    for slot in schema["$defs"]["slots"]["properties"]
        .as_object()
        .unwrap()
        .keys()
    {
        let t=Theme::parse(&json!({"format":2,"id":"x","name":"X","slots":{slot:{"fill":"#123456","states":{"hover":{"fill":"#abcdef"}}}}})).unwrap();
        assert!(t.diagnostics().is_empty(), "{slot}");
        assert_eq!(
            t.slot(slot, &[State::Hover], 1.)
                .unwrap()
                .fill()
                .unwrap()
                .solid(),
            echosight_core::theme::v2::Rgba::parse("#abcdef")
        );
    }
    let examples = json!({"fill":{"radial":{"center":[0.5,0.5],"radius":1,"stops":[["#fff",0],["#000",1]]}},"border":{"color":"#fff","width":2,"style":"dotted"},"bevel":{"light":"#fff","dark":"#000","width":1},"radius":[1,2,3,4],"shadow":[{"x":1,"y":2,"blur":3,"spread":4,"color":"#123456"}],"padding":[1,2],"margin":-1,"size":[10,20],"minSize":5,"height":20,"text":{"color":"#fff","size":12,"weight":500,"family":["Arial"],"transform":"uppercase","letterSpacing":1,"strikethrough":true,"shadow":[]},"ring":{"style":"dashed","inset":1},"image":{"asset":"skin.svg","mode":"nineSlice","insets":[1,1,1,1]},"opacityAffects":false});
    for key in schema["$defs"]["style"]["properties"]
        .as_object()
        .unwrap()
        .keys()
    {
        assert!(examples.get(key).is_some(), "missing property test: {key}");
    }
    let t =
        Theme::parse(&json!({"format":2,"id":"x","name":"X","slots":{"button":examples}})).unwrap();
    let s = t.slot("button", &[], 0.).unwrap();
    assert!(s.fill().is_some());
    assert!(s.text().is_some());
    assert!(s.image().is_some());
    assert!(s.bevel().is_some());
    assert!(s.border().is_some());
    assert!(s.shadows().is_some());
    assert_eq!(s.edges("padding"), Some([1., 2., 1., 2.]));
}
