use serde_json::{Value, json};
use std::{fs, path::Path};

fn fixture() -> Value {
    serde_json::from_str(include_str!("golden/theme-library/pipeline.json")).unwrap()
}
fn write_files(root: &Path, value: &Value) {
    for (name, content) in value.as_object().unwrap() {
        let p = root.join(name);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, content.as_str().unwrap()).unwrap();
    }
}
fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for e in fs::read_dir(source).unwrap() {
        let e = e.unwrap();
        let t = target.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_tree(&e.path(), &t);
        } else {
            fs::copy(e.path(), t).unwrap();
        }
    }
}
fn normalize(value: Value, root: &Path) -> Value {
    serde_json::from_str(
        &serde_json::to_string(&value)
            .unwrap()
            .replace(root.to_str().unwrap(), "$ROOT"),
    )
    .unwrap()
}
fn actual_pipeline(root: &Path, case: &Value, stamp: &str) -> Value {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use echosight_core::theme::{ThemeLibrary, ThemeRoot};
    let mut library = ThemeLibrary::new(root.join("themes")).unwrap();
    library.initialize(&ThemeRoot::new(root.join("defaults")).unwrap(), stamp);
    let first = json!(library.themes);
    let mut deliveries = vec![];
    for request in case["deliveries"].as_array().unwrap() {
        let mut request = request.clone();
        let theme = library.get(request["id"].as_str().unwrap());
        request["result"] = if request["kind"] == "css" {
            json!(library.root().load_css(theme, &request["name"]).unwrap())
        } else {
            json!(library.root().load_asset(theme, &request["name"]).unwrap())
        };
        deliveries.push(request);
    }
    let after = if case["after"].is_object() {
        let mut files = serde_json::Map::new();
        for name in case["after"].as_object().unwrap().keys() {
            files.insert(
                name.clone(),
                json!(STANDARD.encode(fs::read(root.join("themes").join(name)).unwrap())),
            );
        }
        Value::Object(files)
    } else {
        Value::Null
    };
    let result = json!({"themes":first,"warnings":library.warnings,"errors":library.failures.iter().map(|f| &f.context).collect::<Vec<_>>(),"deliveries":deliveries,"after":after});
    library.reload(stamp);
    assert_eq!(json!(library.themes), first);
    result
}

#[test]
fn actual_typescript_filesystem_pipeline() {
    let oracle = fixture();
    for case in oracle["cases"].as_array().unwrap() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join(case["name"].as_str().unwrap());
        let themes = dir.join("themes");
        let defaults = dir.join("defaults");
        fs::create_dir_all(&themes).unwrap();
        fs::create_dir_all(&defaults).unwrap();
        write_files(&themes, &case["before"]);
        write_files(&defaults, &case["bundled"]);
        if case["name"] == "bundled" {
            copy_tree(
                &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/themes"),
                &defaults,
            );
        }
        let actual = normalize(
            actual_pipeline(&dir, case, oracle["stamp"].as_str().unwrap()),
            temp.path(),
        );
        assert_eq!(
            actual,
            json!({"themes":case["themes"],"warnings":case["warnings"],"errors":case["errors"],"deliveries":case["deliveries"],"after":case["after"]}),
            "{}",
            case["name"]
        );
    }
}

#[test]
fn actual_typescript_sound_resolution() {
    for case in fixture()["sounds"].as_array().unwrap() {
        use echosight_core::theme::sounds::{
            can_play_sounds, playback_volume, resolve_theme_sound_definition,
        };
        let theme = case["theme"].as_object().unwrap();
        let definition =
            resolve_theme_sound_definition(Some(theme), case["event"].as_str().unwrap());
        let actual: Value = serde_json::from_str(
            &echosight_core::storage::json_format(&json!(definition)).unwrap(),
        )
        .unwrap();
        assert_eq!(actual, case["definition"]);
        let plays = can_play_sounds(Some(theme), case["settings"]["enabled"].as_bool().unwrap())
            && definition.is_some();
        assert_eq!(plays, !case["played"].as_array().unwrap().is_empty());
        if plays {
            let definition = definition.unwrap();
            assert_eq!(json!([definition.asset_name]), case["requests"]);
            let volume = playback_volume(
                case["settings"]["volume"].as_f64().unwrap(),
                theme,
                &definition,
            );
            assert_eq!(volume, case["played"][0]["volume"].as_f64().unwrap());
        }
    }
}
