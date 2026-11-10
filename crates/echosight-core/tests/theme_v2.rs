use echosight_core::theme::v2::{Severity, State, Theme, ThemeFormat, detect_format};
use serde_json::{Value, json};

fn base() -> Value {
    json!({"format":2,"id":"custom","name":"Custom","tokens":{},"slots":{}})
}
fn load(v: Value) -> Theme {
    Theme::parse(&v).unwrap_or_else(|d| panic!("{d:#?}"))
}

#[test]
fn dispatch_and_malformed_documents() {
    assert_eq!(detect_format(&json!({})).unwrap(), ThemeFormat::V1);
    assert_eq!(detect_format(&base()).unwrap(), ThemeFormat::V2);
    for v in [
        json!(null),
        json!([]),
        json!({"format":3}),
        json!({"format":"2"}),
        json!({"format":2,"id":4,"name":"x"}),
    ] {
        assert!(Theme::parse(&v).is_err(), "{v}");
    }
    for (path, value) in [
        ("/customCSS", json!(null)),
        ("/cssFile", json!("x.css")),
        ("/tokens", json!([])),
        ("/slots", json!(false)),
    ] {
        let mut v = base();
        v.as_object_mut().unwrap().insert(path[1..].into(), value);
        assert!(Theme::parse(&v).is_err());
    }
}

#[test]
fn references_inheritance_states_and_typed_fill() {
    let t = load(
        json!({"format":2,"id":"a","name":"A","tokens":{"colors":{"surface":{"main":"#123456"},"accent":"@surface.main"}},"slots":{
            "button":{"fill":"@colors.accent","text":{"size":12,"color":"#fff"},"states":{"hover":{"text":{"size":14}},"disabled":{"text":{"color":"#888"}}}},
            "button.primary":{"$extends":"button","text":{"weight":700},"states":{"hover":{"text":{"color":"#f00"}},"active":{"radius":4}}}
        }}),
    );
    let s = t
        .slot(
            "button.primary",
            &[State::Active, State::Hover, State::Disabled],
            0.5,
        )
        .unwrap();
    assert_eq!(s.text().unwrap().size, Some(14.));
    assert_eq!(s.text().unwrap().weight, Some(700.));
    assert_eq!(s.properties()["text"]["color"], "#888");
    assert_eq!(s.number("radius"), Some(4.));
    assert_eq!(s.fill().unwrap().solid().unwrap().a, 0.5);
    assert!(t.slot("absent", &[], 1.).is_none());
}

#[test]
fn errors_are_located_deterministic_and_cycle_safe() {
    for (tokens, slots) in [
        (
            json!({"colors":{"a":"@colors.b","b":"@colors.a"}}),
            json!({}),
        ),
        (json!({}), json!({"button":{"fill":"@surface.missing"}})),
        (
            json!({}),
            json!({"button":{"$extends":"input"},"input":{"$extends":"button"}}),
        ),
        (json!({}), json!({"button":{"$extends":"missing"}})),
        (
            json!({"radius":{"a":"@radius.b","b":"wide"}}),
            json!({"button":{"radius":"@radius.a"}}),
        ),
    ] {
        let mut v = base();
        v["tokens"] = tokens;
        v["slots"] = slots;
        let a = Theme::parse(&v).unwrap_err();
        assert_eq!(a, Theme::parse(&v).unwrap_err());
        assert!(
            a.iter()
                .any(|d| d.severity == Severity::Error && !d.location.is_empty())
        );
    }
}

#[test]
fn numeric_ranges_unknown_fields_and_images() {
    for style in [
        json!({"radius":-1}),
        json!({"fill":{"color":"#fff","alpha":1.1}}),
        json!({"text":{"weight":0}}),
        json!({"fill":"rgb(256,0,0)"}),
        json!({"image":{"asset":"../escape.png","mode":"cover"}}),
        json!({"fill":{"gradient":{"angle":90,"stops":[["#fff",1],["#000",0]]}}}),
        json!({"bevel":{"width":-1,"light":"#fff","dark":"#000"}}),
    ] {
        let mut v = base();
        v["slots"] = json!({"button":style});
        assert!(Theme::parse(&v).is_err(), "{v}");
    }
    let mut v = base();
    v["slots"] = json!({"futureSlot":{"futureProperty":7},"button":{"futureProperty":8}});
    let t = load(v);
    assert!(
        t.diagnostics()
            .iter()
            .all(|d| d.severity == Severity::Warning)
    );
    assert!(t.diagnostics().len() >= 3);
    let t = load(
        json!({"format":2,"id":"i","name":"I","tokens":{"bevels":{"raised":{"light":"#fff","dark":"#000","width":1}}},"slots":{"button":{"bevel":"@bevels.raised"}},"images":{"button":{"asset":"normal.svg","mode":"nineSlice","insets":[1,2,3,4],"states":{"hover":{"asset":"hover.svg","mode":"cover"}}}}}),
    );
    let s = t.slot("button", &[State::Hover], 1.).unwrap();
    assert_eq!(s.image().unwrap().asset, "hover.svg");
    assert_eq!(s.bevel().unwrap().width, Some(1.));
}

#[test]
fn opacity_exclusions_and_no_text_fading() {
    let t = load(
        json!({"format":2,"id":"o","name":"O","tokens":{"colors":{"surface":{"a":"rgba(10,20,30,0.8)"}},"opacity":{"appliesTo":["surface.*"]}},"slots":{
            "button":{"fill":"@surface.a","text":{"color":"#fff"}},"input":{"fill":"@surface.a","opacityAffects":false},"header":{"fill":"#fff","opacityAffects":true}
        }}),
    );
    assert!(
        (t.slot("button", &[], 0.5)
            .unwrap()
            .fill()
            .unwrap()
            .solid()
            .unwrap()
            .a
            - 0.4)
            .abs()
            < 1e-9
    );
    assert_eq!(
        t.slot("input", &[], 0.)
            .unwrap()
            .fill()
            .unwrap()
            .solid()
            .unwrap()
            .a,
        0.8
    );
    assert_eq!(
        t.slot("header", &[], 0.)
            .unwrap()
            .fill()
            .unwrap()
            .solid()
            .unwrap()
            .a,
        0.
    );
    assert_eq!(
        t.slot("button", &[], 0.).unwrap().properties()["text"]["color"],
        "#fff"
    );
}

#[test]
fn variant_merge_is_defu_not_style_merge() {
    let t = load(
        json!({"format":2,"id":"v","name":"V","tokens":{"fonts":{"ui":{"family":["Base"]}},"radius":{"a":2}},"slots":{"button":{"radius":"@radius.a"}},"variants":{"red":{"tokens":{"fonts":{"ui":{"family":["Variant"]}},"radius":{"a":null}}}}}),
    );
    let v = t.variant("red").unwrap();
    assert_eq!(v.document()["id"], "v-red");
    assert_eq!(
        v.document()["tokens"]["fonts"]["ui"]["family"],
        json!(["Variant", "Base"])
    );
    assert_eq!(
        v.slot("button", &[], 1.).unwrap().number("radius"),
        Some(2.)
    );
}
