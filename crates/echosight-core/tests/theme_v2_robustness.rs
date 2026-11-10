use echosight_core::theme::v2::{State, Theme};
use serde_json::{Value, json};

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
