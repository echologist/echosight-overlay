use echosight_core::theme::v2::{ConversionInput, State, Theme, convert_v1};
use serde_json::json;

#[test]
fn conversion_is_token_driven_preserves_ids_and_reports_every_loss() {
    let source = json!({"id":"mine-not-a-bundled-id","name":"Mine","colors":{"text":{"primary":"#123456"},"background":{"primary":"rgba(10,20,30,0.8)"}},"fonts":{"primary":{"family":"Arial, sans-serif","sizes":{"task":"13px"},"weights":{"normal":"400"}}},"styles":{"interactive":{"container":{"background":"var(--bg-primary)","padding":"2px 4px","position":"fixed"}}},"customCSS":".modal-content { color: var(--text-primary); background: linear-gradient(90deg, #fff 0%, #000 100%); } .unknown { color: red; padding: 2px; } @import url('secret-font.css');","sounds":{"enabled":true,"events":{"taskCompleted":"ding.wav"}}});
    let before = source.clone();
    let input = ConversionInput {
        document: &source,
        css_file: None,
        asset_files: &[],
    };
    let a = convert_v1(input).unwrap();
    let b = convert_v1(input).unwrap();
    assert_eq!(a, b);
    assert_eq!(source, before);
    assert_eq!(a.theme["id"], source["id"]);
    assert_eq!(a.theme["sounds"], source["sounds"]);
    let t = Theme::parse(&a.theme).unwrap();
    assert_eq!(
        t.slot("taskText", &[], 1.)
            .unwrap()
            .text()
            .unwrap()
            .color
            .as_deref(),
        Some("#123456")
    );
    assert_eq!(
        t.slot("window.interactive", &[], 0.5)
            .unwrap()
            .fill()
            .unwrap()
            .solid()
            .unwrap()
            .a,
        0.4
    );
    assert!(
        a.diagnostics
            .iter()
            .any(|d| d.value.to_string().contains("fixed"))
    );
    assert!(
        a.diagnostics
            .iter()
            .any(|d| d.location.contains(".unknown") && d.value.to_string().contains("color"))
    );
    assert!(
        a.diagnostics
            .iter()
            .any(|d| d.value.to_string().contains("secret-font.css"))
    );
    let mut changed = source.clone();
    changed["colors"]["text"]["primary"] = json!("#abcdef");
    let c = convert_v1(ConversionInput {
        document: &changed,
        ..input
    })
    .unwrap();
    assert_ne!(a.theme, c.theme);
    assert_eq!(
        Theme::parse(&c.theme)
            .unwrap()
            .slot("taskText", &[], 1.)
            .unwrap()
            .text()
            .unwrap()
            .color
            .as_deref(),
        Some("#abcdef")
    );
}

#[test]
fn component_states_assets_variants_and_hex_opacity_are_not_guessed() {
    let source = json!({"id":"x","name":"X","colors":{"background":{"primary":"#fff"}},"components":{"button":{"color":"red","states":{"hover":{"background":"blue"}}}},"styles":{"interactive":{"container":{"background":"var(--bg-primary)"}}},"variants":{"alt":{"colors":{"background":{"primary":"#000"}},"sounds":{"volume":0.3}}}});
    let assets = vec![
        "main_bg.svg".to_string(),
        "button_hover.svg".to_string(),
        "unknown.png".to_string(),
    ];
    let c = convert_v1(ConversionInput {
        document: &source,
        css_file: None,
        asset_files: &assets,
    })
    .unwrap();
    let t = Theme::parse(&c.theme).unwrap();
    assert!(t.slot("button", &[], 1.).unwrap().text().is_none());
    assert_eq!(
        t.slot("button", &[State::Hover], 1.)
            .unwrap()
            .image()
            .unwrap()
            .asset,
        "button_hover.svg"
    );
    assert_eq!(
        t.slot("window.interactive", &[], 0.)
            .unwrap()
            .fill()
            .unwrap()
            .solid()
            .unwrap()
            .a,
        1.
    );
    assert_eq!(
        t.variant("alt")
            .unwrap()
            .slot("window.interactive", &[], 1.)
            .unwrap()
            .fill()
            .unwrap()
            .solid()
            .unwrap()
            .r,
        0.
    );
    assert!(c.diagnostics.iter().any(|d| d.code == "v1-ignored-base"));
    assert!(c.diagnostics.iter().any(|d| d.value == "unknown.png"));
}

#[test]
fn malformed_fonts_variants_and_css_values_are_never_silent() {
    let source = json!({"id":"x","name":"X","fonts":{"primary":false},"customCSS":"button { box-shadow: 0 0 bogus red; }","variants":{"alt":{"mystery":"value"}}});
    let c = convert_v1(ConversionInput {
        document: &source,
        css_file: None,
        asset_files: &[],
    })
    .unwrap();
    assert!(c.diagnostics.iter().any(|d| d.location == "/fonts/primary"));
    assert!(
        c.diagnostics
            .iter()
            .any(|d| d.value.to_string().contains("bogus"))
    );
    assert!(
        c.diagnostics
            .iter()
            .any(|d| d.location == "/variants/alt/mystery")
    );
}

#[test]
fn invalid_inputs_and_unavailable_css_are_explicit() {
    for source in [
        json!(null),
        json!({"id":1,"name":"x"}),
        json!({"format":2,"id":"x","name":"x"}),
    ] {
        assert!(
            convert_v1(ConversionInput {
                document: &source,
                css_file: None,
                asset_files: &[]
            })
            .is_err()
        );
    }
    let source = json!({"id":"x","name":"x","cssFile":"skin.css"});
    let c = convert_v1(ConversionInput {
        document: &source,
        css_file: None,
        asset_files: &[],
    })
    .unwrap();
    assert!(c.provisional);
    assert!(c.diagnostics.iter().any(|d| d.code == "missing-css"));
}
