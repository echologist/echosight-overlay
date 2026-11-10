use echosight_core::theme::v2::{Fill, Severity, State, Theme, ThemeFormat, detect_format};
use serde_json::{Value, json};

fn fill_document(fill: Value) -> Value {
    json!({"format":2,"id":"x","name":"x","slots":{"button":{"fill":fill}}})
}

#[test]
fn accepted_fill_selects_first_valid_branch_not_first_present_key() {
    let linear = json!({"angle":0,"stops":[["red",0],["blue",1]]});
    let radial = json!({"center":[0.5,0.5],"radius":1,"stops":[["red",0],["blue",1]]});
    for (fill, expected) in [
        (json!({"color":null,"gradient":linear}), "linear"),
        (json!({"color":false,"gradient":linear}), "linear"),
        (json!({"color":"not-a-color","gradient":linear}), "linear"),
        (json!({"color":{},"radial":radial}), "radial"),
        (json!({"gradient":null,"radial":radial}), "radial"),
        (json!({"gradient":{"angle":0},"radial":radial}), "radial"),
        (
            json!({"color":"red","gradient":linear,"radial":radial}),
            "solid",
        ),
        (json!({"gradient":linear,"radial":radial}), "linear"),
        (
            json!({"color":"red","gradient":null,"radial":false}),
            "solid",
        ),
        (
            json!({"gradient":linear,"radial":null,"future":{"mode":"nineSlice"}}),
            "linear",
        ),
    ] {
        let theme = Theme::parse(&fill_document(fill.clone())).unwrap();
        let resolved = theme.slot("button", &[], 0.5).unwrap();
        let actual = match resolved.fill().expect("accepted fill must render") {
            Fill::Solid(_) => "solid",
            Fill::Linear { .. } => "linear",
            Fill::Radial { .. } => "radial",
        };
        assert_eq!(actual, expected, "{fill}");
        assert_eq!(resolved.properties()["fill"], fill);
        assert!(
            theme
                .diagnostics()
                .iter()
                .all(|d| d.severity == Severity::Warning)
        );
    }
    for fill in [
        json!({"color":null}),
        json!({"gradient":null,"radial":false}),
    ] {
        let document = fill_document(fill);
        let errors = Theme::parse(&document).unwrap_err();
        assert_eq!(errors, Theme::parse(&document).unwrap_err());
        assert!(errors.iter().any(|d| d.location == "/slots/button/fill"));
    }
}

#[test]
fn unknown_properties_are_not_semantically_interpreted() {
    for future in [
        json!({"mode":"nineSlice"}),
        json!({"stops":[["red",1],["blue",0]]}),
        json!({"nested":{"mode":"nineSlice","stops":[["red",1],["blue",0]]}}),
    ] {
        let document = json!({"format":2,"id":"x","name":"x","slots":{"button":{"fill":"red","future":future}}});
        let theme = Theme::parse(&document).unwrap();
        assert!(
            theme
                .diagnostics()
                .iter()
                .any(|d| d.location == "/slots/button/future" && d.code == "unknown")
        );
        assert!(
            theme
                .diagnostics()
                .iter()
                .all(|d| d.severity == Severity::Warning)
        );
        assert_eq!(
            theme.slot("button", &[], 1.).unwrap().properties()["future"],
            future
        );
    }
    for (style, location, code) in [
        (
            json!({"image":{"asset":"x.png","mode":"nineSlice"}}),
            "/slots/button/image",
            "insets",
        ),
        (
            json!({"fill":{"gradient":{"angle":0,"stops":[["red",1],["blue",0]]}}}),
            "/slots/button/fill/gradient/stops/1",
            "stop-order",
        ),
        (
            json!({"fill":{"radial":{"center":[0.5,0.5],"radius":1,"stops":[["red",1],["blue",0]]}}}),
            "/slots/button/fill/radial/stops/1",
            "stop-order",
        ),
    ] {
        let document = json!({"format":2,"id":"x","name":"x","slots":{"button":style}});
        let errors = Theme::parse(&document).unwrap_err();
        assert_eq!(errors, Theme::parse(&document).unwrap_err());
        assert!(
            errors
                .iter()
                .any(|d| d.location == location && d.code == code),
            "{errors:?}"
        );
    }
}

#[test]
fn unknown_fill_literals_do_not_trigger_opacity_or_reference_traversal() {
    for future in [
        json!("@surface.foo"),
        json!({"nested":["@surface.foo","@missing"]}),
    ] {
        let theme = Theme::parse(&fill_document(json!({"color":"red","future":future}))).unwrap();
        let resolved = theme.slot("button", &[], 0.5).unwrap();
        assert_eq!(resolved.fill().unwrap().solid().unwrap().a, 1.);
        assert_eq!(resolved.properties()["fill"]["future"], future);
    }
    let theme = Theme::parse(&json!({"format":2,"id":"x","name":"x","tokens":{"colors":{"surface":{"foo":"red"},"alias":"@surface.foo","chain":"@colors.alias"}},"slots":{"button":{"fill":{"color":"@colors.chain","future":"@missing"}}}})).unwrap();
    assert_eq!(
        theme
            .slot("button", &[], 0.5)
            .unwrap()
            .fill()
            .unwrap()
            .solid()
            .unwrap()
            .a,
        0.5
    );
}

#[test]
fn format_numbers_follow_json_schema_numeric_equality() {
    for number in ["2", "2.0", "2e0"] {
        let document: Value =
            serde_json::from_str(&format!(r#"{{"format":{number},"id":"x","name":"x"}}"#)).unwrap();
        assert!(Theme::parse(&document).is_ok(), "{number}");
        assert_eq!(detect_format(&document).unwrap(), ThemeFormat::V2);
    }
    for number in ["1", "1.0", "1e0"] {
        let document: Value =
            serde_json::from_str(&format!(r#"{{"format":{number},"id":"x","name":"x"}}"#)).unwrap();
        assert_eq!(detect_format(&document).unwrap(), ThemeFormat::V1);
        assert!(Theme::parse(&document).is_err());
    }
    for value in [
        json!(0),
        json!(-2),
        json!(2.5),
        json!(2.0000000000000004),
        json!(1e300),
        json!("2"),
        json!(true),
        Value::Null,
        json!([]),
        json!({}),
    ] {
        let document = json!({"format":value,"id":"x","name":"x"});
        assert!(detect_format(&document).is_err(), "{value}");
        assert!(Theme::parse(&document).is_err(), "{value}");
    }
}

#[test]
fn selected_fill_branch_alone_controls_references_semantics_and_opacity() {
    let document = fill_document(json!({
        "gradient":{"angle":0,"stops":[["@surface.missing",1],["blue",0]]},
        "radial":{"mode":"nineSlice"},
        "color":"red"
    }));
    let theme = Theme::parse(&document).unwrap();
    let style = theme.slot("button", &[], 0.5).unwrap();
    assert_eq!(
        style.fill().unwrap().solid(),
        echosight_core::theme::v2::Rgba::parse("red")
    );
    assert_eq!(
        style.properties()["fill"],
        document["slots"]["button"]["fill"]
    );
    for key in ["gradient", "radial"] {
        assert!(
            theme
                .diagnostics()
                .iter()
                .any(|d| d.location == format!("/slots/button/fill/{key}") && d.code == "unknown")
        );
    }
}

#[test]
fn selected_branch_errors_are_not_hidden_by_other_valid_branches() {
    let gradient = json!({"angle":0,"stops":[["red",0],["blue",1]]});
    let radial = json!({"center":[0.5,0.5],"radius":1,"stops":[["red",0],["blue",1]]});
    for (fill, location, code) in [
        (
            json!({"color":"@missing","gradient":gradient}),
            "/slots/button/fill/color",
            "missing-reference",
        ),
        (
            json!({"gradient":{"angle":0,"stops":[["red",1],["blue",0]]},"radial":radial}),
            "/slots/button/fill/gradient/stops/1",
            "stop-order",
        ),
    ] {
        let document = fill_document(fill);
        let errors = Theme::parse(&document).unwrap_err();
        assert_eq!(errors, Theme::parse(&document).unwrap_err());
        assert!(
            errors
                .iter()
                .any(|d| d.location == location && d.code == code),
            "{errors:?}"
        );
    }
}

#[test]
fn reference_expansion_retains_destination_semantic_checks() {
    for (style, field, location, code) in [
        (
            json!({"asset":"x.png","mode":"nineSlice"}),
            "image",
            "/slots/button/image",
            "insets",
        ),
        (
            json!({"gradient":{"angle":0,"stops":[["red",1],["blue",0]]}}),
            "fill",
            "/slots/button/fill/gradient/stops/1",
            "stop-order",
        ),
        (
            json!({"radial":{"center":[0.5,0.5],"radius":1,"stops":[["red",1],["blue",0]]}}),
            "fill",
            "/slots/button/fill/radial/stops/1",
            "stop-order",
        ),
    ] {
        // Future token storage is opaque until referenced from a supported destination.
        let mut document = json!({"format":2,"id":"x","name":"x","tokens":{"future":{"shape":style,"alias":"@future.shape"}}});
        assert!(Theme::parse(&document).is_ok());
        document["slots"] = json!({"button":{field:"@future.alias"}});
        let errors = Theme::parse(&document).unwrap_err();
        assert_eq!(errors, Theme::parse(&document).unwrap_err());
        assert!(
            errors
                .iter()
                .any(|d| d.location == location && d.code == code),
            "{errors:?}"
        );
    }
    let document = json!({"format":2,"id":"x","name":"x","tokens":{"future":{"shape":{"color":null,"gradient":{"angle":0,"stops":[["@colors.alias",0],["blue",1]]},"future":"@missing"},"alias":"@future.shape"},"colors":{"alias":"@surface.red","surface":{"red":"red"}}},"slots":{"button":{"fill":"@future.alias"}}});
    let theme = Theme::parse(&document).unwrap();
    let Fill::Linear { angle, stops } = theme.slot("button", &[], 0.5).unwrap().fill().unwrap()
    else {
        panic!("expected referenced linear fill");
    };
    assert_eq!(angle, 0.);
    assert_eq!(
        stops.iter().map(|s| s.position).collect::<Vec<_>>(),
        vec![0., 1.]
    );
    assert_eq!(
        stops.iter().map(|s| s.color.a).collect::<Vec<_>>(),
        vec![0.5, 0.5]
    );
}

#[test]
fn oversized_diagnostic_values_are_omitted_not_rejected() {
    let future = json!({"text":"é".repeat(4096), "items":vec![0; 100]});
    let doc = fill_document(json!("red"));
    let mut doc = doc;
    doc["slots"]["button"]["future"] = future.clone();
    let theme = Theme::parse(&doc).unwrap();
    assert!(theme.diagnostics().iter().all(|d| d.value.is_null()));
    assert!(
        theme
            .diagnostics()
            .iter()
            .all(|d| d.message.contains("omitted"))
    );
    assert_eq!(
        theme.slot("button", &[], 1.).unwrap().properties()["future"],
        future
    );
}

#[test]
fn diagnostic_count_exhaustion_is_a_bounded_error() {
    let mut doc = fill_document(json!("red"));
    for i in 0..2000 {
        doc[format!("future{i}")] = json!({"nested": "value"});
    }
    let Err(errors) = Theme::parse(&doc) else {
        panic!("bounded diagnostic collection");
    };
    assert!(errors.len() <= 512);
    assert!(
        errors
            .iter()
            .any(|d| d.code == "budget" && d.severity == Severity::Error)
    );
}

#[test]
fn inherited_string_bytes_are_bounded_even_with_only_declared_slots() {
    let schema: Value = serde_json::from_str(echosight_core::theme::v2::SCHEMA).unwrap();
    let mut doc = fill_document(json!("red"));
    doc["slots"]["button"]["future"] = json!("é".repeat(128 * 1024));
    for name in schema["$defs"]["slots"]["properties"]
        .as_object()
        .unwrap()
        .keys()
    {
        if name != "button" {
            doc["slots"][name] = json!({"$extends":"button"});
        }
    }
    let Err(errors) = Theme::parse(&doc) else {
        panic!("copied UTF-8 bytes must be bounded");
    };
    assert!(
        errors
            .iter()
            .any(|d| d.code == "budget" && d.message.contains("materialization"))
    );
    // Below the byte allowance, the same opaque inherited field is retained.
    doc["slots"]["button"]["future"] = json!("é".repeat(16 * 1024));
    let theme = Theme::parse(&doc).unwrap();
    assert!(
        theme
            .diagnostics()
            .iter()
            .all(|d| d.severity == Severity::Warning)
    );
    for name in theme.slot_names() {
        assert_eq!(
            theme.slot(name, &[], 1.).unwrap().properties()["future"],
            doc["slots"]["button"]["future"]
        );
    }
}

#[test]
fn variant_diagnostic_aggregation_cannot_bypass_the_cap() {
    let mut doc = fill_document(json!("red"));
    doc["future"] = json!(true);
    doc["variants"] = json!({});
    for i in 0..600 {
        doc["variants"][format!("v{i}")] = json!({});
    }
    let Err(errors) = Theme::parse(&doc) else {
        panic!("bounded variant diagnostics");
    };
    assert!(errors.len() <= 512);
    assert!(
        errors
            .iter()
            .any(|d| d.code == "budget" && d.severity == Severity::Error)
    );
}

#[test]
fn ordinary_unknown_inheritance_and_variants_stay_warning_only() {
    let doc = json!({"format":2,"id":"x","name":"x","slots":{
        "button":{"fill":"red","future":{"values":[0,1,2],"literal":"@missing"}},
        "future.slot":{"$extends":"button"}},"variants":{"blue":{"slots":{"button":{"fill":"blue"}}}}});
    let theme = Theme::parse(&doc).unwrap();
    assert!(
        theme
            .diagnostics()
            .iter()
            .all(|d| d.severity == Severity::Warning)
    );
    for t in [theme.clone(), theme.variant("blue").unwrap()] {
        assert_eq!(
            t.slot("future.slot", &[], 1.).unwrap().properties()["future"],
            doc["slots"]["button"]["future"]
        );
    }
}

#[test]
fn malformed_nested_values_never_panic() {
    let values = [
        Value::Null,
        json!(true),
        json!(0),
        json!(-1),
        json!("nonsense"),
        json!([]),
        json!({}),
    ];
    let paths = [
        "/tokens",
        "/tokens/colors",
        "/tokens/fonts",
        "/tokens/opacity",
        "/slots",
        "/slots/button",
        "/slots/button/states",
        "/slots/button/text",
        "/slots/button/image",
        "/images",
        "/sounds",
        "/variants",
    ];
    for path in paths {
        for value in &values {
            let mut doc = json!({"format":2,"id":"x","name":"X","tokens":{"colors":{},"fonts":{},"opacity":{}},"slots":{"button":{"states":{},"text":{},"image":{"asset":"a.svg","mode":"stretch"}}},"images":{},"sounds":{},"variants":{}});
            *doc.pointer_mut(path).unwrap() = value.clone();
            assert!(
                std::panic::catch_unwind(|| Theme::parse(&doc)).is_ok(),
                "panic for {path}: {value}"
            );
        }
    }
}

#[test]
fn referenced_opacity_and_state_union_replacement() {
    let t=Theme::parse(&json!({"format":2,"id":"x","name":"X","tokens":{"opacity":{"interactive":"@opacity.clickThrough","clickThrough":0.4}},"slots":{"window.interactive":{"fill":"#fff"},"button":{"fill":{"gradient":{"angle":90,"stops":[["#fff",0],["#000",1]]}},"states":{"hover":{"fill":{"color":"#f00","alpha":0.8}}}}}})).unwrap();
    assert_eq!(
        t.slot("window.interactive", &[], 0.5)
            .unwrap()
            .fill()
            .unwrap()
            .solid()
            .unwrap()
            .a,
        0.2
    );
    let s = t.slot("button", &[State::Hover], 1.).unwrap();
    assert!(s.properties()["fill"].get("gradient").is_none());
    assert_eq!(s.fill().unwrap().solid().unwrap().a, 0.8);
}

#[test]
fn metadata_and_asset_names_are_not_token_references() {
    let doc = json!({"format":2,"id":"@theme","name":"@name","author":"@author","description":"@mention","tokens":{"fonts":{"ui":{"family":["@Font"],"file":"@font.ttf"}}},"sounds":{"events":{"undo":"@sound.wav"}},"slots":{"button":{"image":{"asset":"@skin.svg","mode":"cover"}}}});
    assert!(Theme::parse(&doc).is_ok());
}

#[test]
fn unknown_reference_like_values_warn_but_token_names_still_resolve() {
    let doc = json!({"format":2,"id":"x","name":"X","future":"@missing","tokens":{"radius":{"name":"@radius.other","other":3}},"slots":{"button":{"future":"@missing","radius":"@radius.name"}}});
    let t = Theme::parse(&doc).unwrap();
    assert_eq!(
        t.slot("button", &[], 1.).unwrap().number("radius"),
        Some(3.)
    );
    let looped =
        json!({"format":2,"id":"x","name":"X","tokens":{"radius":{"name":"@radius.name"}}});
    assert!(Theme::parse(&looped).is_err());
}

#[test]
fn reference_graph_and_unknown_tree_have_bounded_depth() {
    let mut deep = json!(0);
    for _ in 0..200 {
        deep = json!({"nested":deep});
    }
    let doc = json!({"format":2,"id":"x","name":"X","future":deep});
    assert!(Theme::parse(&doc).is_err());
    let mut colors = json!({"a":"#fff"});
    for i in 0..70 {
        colors[format!("x{i}")] = json!(if i == 69 {
            "#fff".into()
        } else {
            format!("@colors.x{}", i + 1)
        });
    }
    assert!(
        Theme::parse(&json!({"format":2,"id":"x","name":"X","tokens":{"colors":colors}})).is_err()
    );
}
