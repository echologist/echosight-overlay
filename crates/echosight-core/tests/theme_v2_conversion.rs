use echosight_core::theme::v2::{ConversionInput, State, Theme, convert_v1};
use serde_json::{Value, json};

fn convert(source: &Value) -> echosight_core::theme::v2::Conversion {
    let before = source.clone();
    let conversion = convert_v1(ConversionInput {
        document: source,
        css_file: None,
        asset_files: &[],
    })
    .unwrap();
    assert_eq!(*source, before);
    conversion
}

// Source-observed regressions, not a second CSS implementation. See the provenance
// table in docs/plans/gpui-port/parity-theme-v2.md: unchanged TS generator + Chrome
// observations on 889c02f underpin the expected colors, fonts, shadows and scaling.
#[test]
fn observed_opaque_variant_clears_generated_user_opacity() {
    let c = convert(&json!({"id":"x","name":"X",
        "styles":{"interactive":{"container":{"background":"rgba(10,20,30,var(--user-transparency))"}}},
        "variants":{"opaque":{"styles":{"interactive":{"container":{"background":"#ffffff"}}}}}}));
    let t = Theme::parse(&c.theme).unwrap();
    assert_eq!(
        t.slot("window.interactive", &[], 0.25)
            .unwrap()
            .fill()
            .unwrap()
            .solid()
            .unwrap()
            .a,
        0.25
    );
    assert_eq!(
        t.variant("opaque")
            .unwrap()
            .slot("window.interactive", &[], 0.25)
            .unwrap()
            .fill()
            .unwrap()
            .solid()
            .unwrap()
            .a,
        1.0
    );
    assert!(c.diagnostics.is_empty());
}

#[test]
fn observed_background_color_keeps_gradient_and_reports_unrepresentable_underlay() {
    for (source, slot, location) in [
        (
            json!({"id":"x","name":"X","components":{"button":{"background":"linear-gradient(red, blue)","backgroundColor":"green"}}}),
            "button",
            "/components/button/backgroundColor",
        ),
        (
            json!({"id":"x","name":"X","customCSS":".header { background:linear-gradient(red,blue); background-color:green; }"}),
            "header",
            "/customCSS/0/.header/background-color",
        ),
    ] {
        let c = convert(&source);
        let t = Theme::parse(&c.theme).unwrap();
        assert!(t.slot(slot, &[], 1.).unwrap().fill().is_some());
        assert!(
            c.theme["slots"][slot]["fill"]["gradient"].is_object(),
            "Chrome retains backgroundImage, not a solid green replacement"
        );
        assert!(
            c.diagnostics
                .iter()
                .any(|d| d.code == "background-layer-loss"
                    && d.location == location
                    && d.value.to_string().contains("green")),
            "v2 has no gradient + background-color underlay"
        );
    }
}

#[test]
fn background_longhand_respects_prior_rules_and_shorthand_resets() {
    let c = convert(
        &json!({"id":"x","name":"X","customCSS":".header { background:linear-gradient(red, blue); } .header { background-color:green; }"}),
    );
    assert!(c.theme["slots"]["header"]["fill"]["gradient"].is_object());
    assert!(
        c.diagnostics
            .iter()
            .any(|d| d.code == "background-layer-loss"
                && d.location == "/customCSS/1/.header/background-color")
    );
    for props in [
        json!({"backgroundColor":"green","background":"linear-gradient(red,blue)"}),
        json!({"background":"red","backgroundColor":"green"}),
    ] {
        let c = convert(&json!({"id":"x","name":"X","components":{"button":props}}));
        assert!(
            !c.diagnostics
                .iter()
                .any(|d| d.code == "background-layer-loss")
        );
    }
}

#[test]
fn observed_json_simultaneous_state_order_loss_is_specific() {
    // Raw JSON pins active-before-hover insertion order; Chrome's forced active+hover is blue.
    let source = serde_json::from_str(r#"{"id":"x","name":"X","components":{"button":{"states":{"active":{"background":"red"},"hover":{"background":"blue"}}}}}"#).unwrap();
    let c = convert(&source);
    let t = Theme::parse(&c.theme).unwrap();
    assert!(
        t.slot("button", &[State::Hover, State::Active], 1.)
            .unwrap()
            .fill()
            .is_some()
    );
    assert!(
        c.diagnostics
            .iter()
            .any(|d| d.code == "simultaneous-state-order"
                && d.location == "/components/button/states"
                && d.value["sourceOrder"] == json!(["active", "hover"])
                && d.value["sourceResult"]["fill"] == "blue"
                && d.value["v2Result"]["fill"] == "red"),
        "fixed v2 precedence cannot encode this JSON source order"
    );
    for states in [
        json!({"hover":{"background":"blue"},"active":{"background":"red"}}),
        json!({"active":{"color":"red"},"hover":{"background":"blue"}}),
        json!({"active":{"background":"red"},"hover":{"background":"red"}}),
    ] {
        let c = convert(&json!({"id":"x","name":"X","components":{"button":{"states":states}}}));
        assert!(
            !c.diagnostics
                .iter()
                .any(|d| d.code == "simultaneous-state-order")
        );
    }
}

#[test]
fn observed_plain_button_font_size_wins_with_class_selector_loss_reported() {
    let c = convert(
        &json!({"id":"x","name":"X","fonts":{"primary":{"sizes":{"ui":"12px"}}},"components":{"button":{"fontSize":"20px"}}}),
    );
    let t = Theme::parse(&c.theme).unwrap();
    assert_eq!(
        t.slot("button", &[], 1.).unwrap().text().unwrap().size,
        Some(12.)
    );
    assert!(
        c.diagnostics
            .iter()
            .any(|d| d.code == "font-selector-precedence"
                && d.location == "/components/button/fontSize"
                && d.value.to_string().contains("20px")),
        "Chrome .template-btn stays 20px: one button slot cannot preserve both selectors"
    );
    let c = convert(&json!({"id":"x","name":"X","components":{"button":{"fontSize":"20px"}}}));
    assert_eq!(
        Theme::parse(&c.theme)
            .unwrap()
            .slot("button", &[], 1.)
            .unwrap()
            .text()
            .unwrap()
            .size,
        Some(20.)
    );
    assert!(
        !c.diagnostics
            .iter()
            .any(|d| d.code == "font-selector-precedence")
    );
}

#[test]
fn observed_effects_reach_known_generated_targets_and_report_utility_classes() {
    let c = convert(
        &json!({"id":"x","name":"X","effects":{"borderRadius":"9px","bevelInset":"1px 2px red","textShadow":"1px 1px blue"}}),
    );
    let t = Theme::parse(&c.theme).unwrap();
    for slot in ["input", "textarea", "select"] {
        let s = t.slot(slot, &[], 1.).unwrap();
        assert_eq!(
            s.shadows().unwrap_or_default().len(),
            1,
            "{slot}: Chrome applies the supported OUTER bevelInset shadow"
        );
        let shadow = s.shadows().unwrap().remove(0);
        assert_eq!((shadow.x, shadow.y, shadow.color.as_str()), (1., 2., "red"));
        assert_eq!(s.edges("radius"), Some([9.; 4]));
        assert_eq!(c.theme["slots"][slot]["radius"], "@radius.default");
    }
    for slot in [
        "button",
        "button.icon",
        "dialog.button.primary",
        "dialog.button.secondary",
    ] {
        assert_eq!(c.theme["slots"][slot]["radius"], "@radius.default");
        assert!(
            !t.slot(slot, &[], 1.)
                .unwrap()
                .text()
                .unwrap()
                .shadow
                .unwrap_or_default()
                .is_empty(),
            "{slot}"
        );
    }
    for effect in ["bevelInset", "textShadow"] {
        assert!(
            c.diagnostics
                .iter()
                .any(|d| d.code == "effect-utility-selector"
                    && d.location == format!("/effects/{effect}"))
        );
    }
}

#[test]
fn observed_titlebar_asset_uses_cover_not_stretch() {
    let source = json!({"id":"x","name":"X"});
    let c = convert_v1(ConversionInput {
        document: &source,
        css_file: None,
        asset_files: &["titlebar_bg.svg".into()],
    })
    .unwrap();
    assert_eq!(
        Theme::parse(&c.theme)
            .unwrap()
            .slot("header", &[], 1.)
            .unwrap()
            .image()
            .unwrap()
            .mode,
        "cover"
    );
    assert!(c.diagnostics.is_empty());
}

#[test]
fn observed_non_record_states_do_not_suppress_component_base() {
    for states in [
        Value::Null,
        json!([]),
        json!([{}]),
        json!(false),
        json!(3),
        json!("hover"),
    ] {
        let c = convert(
            &json!({"id":"x","name":"X","components":{"button":{"color":"red","states":states}}}),
        );
        assert!(!c.diagnostics.iter().any(|d| d.code == "v1-ignored-base"));
        assert_eq!(
            Theme::parse(&c.theme)
                .unwrap()
                .slot("button", &[], 1.)
                .unwrap()
                .text()
                .unwrap()
                .color
                .as_deref(),
            Some("red")
        );
    }
}

#[test]
fn observed_valid_css_trivia_is_not_a_malformed_declaration() {
    for css in [
        "button { color: red; }",
        "button { ;  ; color:red;\n\t }",
        "button { /* comment */ }",
    ] {
        let c = convert(&json!({"id":"x","name":"X","customCSS":css}));
        assert!(
            !c.diagnostics
                .iter()
                .any(|d| d.code == "malformed-declaration")
        );
        assert!(c.diagnostics.iter().any(|d| d.code == "css-source-order"));
    }
    let c = convert(&json!({"id":"x","name":"X","customCSS":"button { color:red; broken; }"}));
    assert!(
        c.diagnostics
            .iter()
            .any(|d| d.code == "malformed-declaration" && d.value == "broken")
    );
}

#[test]
fn opacity_reset_still_preserves_transparent_token_variants_and_state_fills() {
    let c = convert(&json!({"id":"x","name":"X",
        "colors":{"background":{"primary":"rgba(10,20,30,0.8)"}},
        "styles":{"interactive":{"container":{"background":"rgba(10,20,30,var(--user-transparency))"}}},
        "variants":{"token":{"styles":{"interactive":{"container":{"background":"var(--bg-primary)"}}}}},
        "customCSS":"button { background:rgba(10,20,30,var(--user-transparency)); } button:hover { background:#fff; }"}));
    let t = Theme::parse(&c.theme).unwrap();
    assert_eq!(
        t.variant("token")
            .unwrap()
            .slot("window.interactive", &[], 0.25)
            .unwrap()
            .fill()
            .unwrap()
            .solid()
            .unwrap()
            .a,
        0.2
    );
    assert_eq!(
        t.slot("button", &[State::Hover], 0.25)
            .unwrap()
            .fill()
            .unwrap()
            .solid()
            .unwrap()
            .a,
        1.
    );
}

#[test]
fn variant_omissions_and_array_replacement_remain_explicit() {
    let c = convert(&json!({"id":"x","name":"X",
        "components":{"button":{"color":"red"}},
        "variants":{"states":{"components":{"button":{"states":{"hover":{"background":"blue"}}}}}}}));
    assert!(
        c.diagnostics
            .iter()
            .any(|d| d.code == "variant-inherited-value"
                && d.location == "/variants/states/slots/button/text")
    );
    let c = convert(
        &json!({"id":"x","name":"X","effects":{"textShadow":"1px 1px red"},"variants":{"none":{"effects":{"textShadow":"none"}}}}),
    );
    assert!(
        c.diagnostics
            .iter()
            .any(|d| d.code == "variant-array-replacement")
    );
}

#[test]
fn source_oracle_delimiter_and_font_face_controls_remain_reported() {
    let c = convert(
        &json!({"id":"x","name":"X","customCSS":"/*trivia*/button, .header:hover { color: rgb(1, 2, 3); background-image: url(\"a;b,c{d}.png\"); } @font-face { font-family: \"A;B\"; src: url(\"font.woff2\"); }"}),
    );
    assert_eq!(
        c.diagnostics
            .iter()
            .filter(|d| d.code == "unsupported-property"
                && d.value == json!({"background-image":"url(\"a;b,c{d}.png\")"}))
            .count(),
        2
    );
    assert!(c.diagnostics.iter().any(|d| {
        d.code == "unsupported-rule"
            && d.value
                .as_str()
                .is_some_and(|s| s.contains("font.woff2") && s.contains("A;B"))
    }));
    assert!(
        !c.diagnostics
            .iter()
            .any(|d| d.code == "malformed-declaration")
    );
}

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

// Additional quality cases observed through unchanged TS + Chrome at 0db6602.
// The migration ledger records source provenance and the bounded loss semantics.
#[test]
fn observed_mixed_gradient_opacity_reports_per_stop_loss() {
    use echosight_core::theme::v2::Fill;
    for (background, expected_alphas, source_mask) in [
        (
            "linear-gradient(var(--bg-primary), red)",
            vec![0.2, 0.25],
            vec![true, false],
        ),
        (
            "linear-gradient(rgba(10,20,30,var(--user-transparency)), red)",
            vec![0.25, 0.25],
            vec![true, false],
        ),
        (
            "linear-gradient(90deg, red 0%, var(--missing, var(--bg-primary)) 100%)",
            vec![0.25, 0.2],
            vec![false, true],
        ),
    ] {
        for css_source in [false, true] {
            let mut source = json!({"id":"x","name":"X","colors":{"background":{"primary":"rgba(10,20,30,0.8)"}}});
            let location = if css_source {
                source["customCSS"] = json!(format!("button {{ background: {background}; }}"));
                "/customCSS/0/button/background"
            } else {
                source["components"] = json!({"button":{"background":background}});
                "/components/button/background"
            };
            let c = convert(&source);
            let t = Theme::parse(&c.theme).unwrap();
            let Some(Fill::Linear { stops, .. }) = t.slot("button", &[], 0.25).unwrap().fill()
            else {
                panic!("expected typed gradient");
            };
            assert_eq!(
                stops.iter().map(|s| s.color.a).collect::<Vec<_>>(),
                expected_alphas
            );
            let loss = c
                .diagnostics
                .iter()
                .find(|d| d.code == "mixed-gradient-opacity")
                .expect("v1 fades only selected stops; v2 fades the whole fill");
            assert_eq!(loss.location, location);
            assert_eq!(loss.value["source"], background);
            assert_eq!(loss.value["sourceStopOpacityAffects"], json!(source_mask));
            assert_eq!(loss.value["v2StopOpacityAffects"], json!([true, true]));
        }
    }
    // Known variables ignore unused fallbacks, including user-transparency text.
    for (background, expected) in [
        (
            "linear-gradient(var(--bg-primary), var(--bg-secondary))",
            vec![0.2, 0.25],
        ),
        ("linear-gradient(white, red)", vec![1., 1.]),
        (
            "linear-gradient(var(--bg-opaque, rgba(0,0,0,var(--user-transparency))), red)",
            vec![1., 1.],
        ),
        (
            "linear-gradient(var(--missing, rgba(0,0,0,var(--user-transparency))), var(--bg-primary))",
            vec![0.25, 0.2],
        ),
    ] {
        let c = convert(
            &json!({"id":"x","name":"X","colors":{"background":{"primary":"rgba(10,20,30,0.8)","secondary":"rgb(40,50,60)","opaque":"#fff"}},"components":{"button":{"background":background}}}),
        );
        assert!(
            !c.diagnostics
                .iter()
                .any(|d| d.code == "mixed-gradient-opacity")
        );
        let t = Theme::parse(&c.theme).unwrap();
        let Some(Fill::Linear { stops, .. }) = t.slot("button", &[], 0.25).unwrap().fill() else {
            panic!("expected typed gradient")
        };
        assert_eq!(
            stops.iter().map(|s| s.color.a).collect::<Vec<_>>(),
            expected,
            "{background}"
        );
    }
}

#[test]
fn observed_empty_gradient_items_are_rejected_without_repair() {
    for background in [
        "linear-gradient(red,,blue)",
        "linear-gradient(red,blue,)",
        "linear-gradient(,red,blue)",
        "linear-gradient(red, ,blue)",
        "linear-gradient(90deg,,red,blue)",
    ] {
        for (source, location) in [
            (
                json!({"id":"x","name":"X","components":{"button":{"background":background}}}),
                "/components/button/background",
            ),
            (
                json!({"id":"x","name":"X","customCSS":format!("button {{ background:{background}; }}")}),
                "/customCSS/0/button/background",
            ),
        ] {
            let c = convert(&source);
            assert!(
                c.theme["slots"]["button"].get("fill").is_none(),
                "invalid CSS must not become a repaired gradient: {background}"
            );
            assert!(
                c.diagnostics
                    .iter()
                    .any(|d| d.code == "unsupported-property"
                        && d.location == location
                        && d.value == json!({"background":background}))
            );
        }
    }
    let c = convert(
        &json!({"id":"x","name":"X","customCSS":"button { ; background: linear-gradient(rgb(1, 2, 3), rgba(4, 5, 6, 0.5)); padding:  2px   4px; font-family: 'A,B', serif; ; }"}),
    );
    let t = Theme::parse(&c.theme).unwrap();
    assert!(t.slot("button", &[], 1.).unwrap().fill().is_some());
    assert_eq!(
        t.slot("button", &[], 1.).unwrap().edges("padding"),
        Some([2., 4., 2., 4.])
    );
    assert_eq!(
        c.theme["slots"]["button"]["text"]["family"],
        json!(["A,B", "serif"])
    );
    assert!(!c.diagnostics.iter().any(|d| matches!(
        d.code.as_str(),
        "malformed-declaration" | "unsupported-property"
    )));
}

#[test]
fn observed_duplicate_css_properties_keep_final_occurrence_order() {
    for (body, property, expected) in [
        (
            "background:red; background-color:green; background:blue;",
            "fill",
            json!("blue"),
        ),
        (
            "background-color:red; background:green; background-color:blue;",
            "fill",
            json!("blue"),
        ),
        (
            "border:1px solid red; border-color:green; border:2px solid blue;",
            "border",
            json!({"width":2.,"style":"solid","color":"blue"}),
        ),
        (
            "border-color:red; border:1px solid green; border-color:blue;",
            "border",
            json!({"width":1.,"style":"solid","color":"blue"}),
        ),
    ] {
        let c = convert(&json!({"id":"x","name":"X","customCSS":format!("button {{{body}}}")}));
        let t = Theme::parse(&c.theme).unwrap();
        assert_eq!(
            t.slot("button", &[], 1.).unwrap().properties()[property],
            expected,
            "{body}"
        );
        let d = c
            .diagnostics
            .iter()
            .find(|d| d.code == "duplicate-declaration")
            .unwrap();
        assert!(d.message.contains("final-occurrence order"));
        assert!(d.message.contains("invalid-value fallback") && d.message.contains("!important"));
    }
    let c = convert(
        &json!({"id":"x","name":"X","customCSS":"button {background:red !important; background:blue;}"}),
    );
    assert!(
        c.diagnostics
            .iter()
            .any(|d| d.code == "css-source-order" && d.message.contains("importance"))
    );
    assert!(
        c.diagnostics
            .iter()
            .any(|d| d.code == "duplicate-declaration")
    );
    let c = convert(
        &json!({"id":"x","name":"X","customCSS":"button {background:red; background:bogus;}"}),
    );
    assert!(
        c.diagnostics
            .iter()
            .any(|d| d.code == "duplicate-declaration"
                && d.message.contains("invalid-value fallback")
                && d.value == json!({"background":"red"}))
    );
    assert!(
        c.diagnostics
            .iter()
            .any(|d| d.code == "unsupported-property"
                && d.location == "/customCSS/0/button/background"
                && d.value == json!({"background":"bogus"}))
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
