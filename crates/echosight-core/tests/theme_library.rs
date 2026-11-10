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
fn map_strings(value: &mut Value, f: &impl Fn(&str) -> String) {
    match value {
        Value::String(s) => *s = f(s),
        Value::Array(a) => a.iter_mut().for_each(|v| map_strings(v, f)),
        Value::Object(o) => o.values_mut().for_each(|v| map_strings(v, f)),
        _ => {}
    }
}
fn normalize(mut value: Value, root: &Path) -> Value {
    map_strings(&mut value, &|s| {
        let s = s.replace(root.to_str().unwrap(), "$ROOT");
        if s.starts_with("$ROOT") {
            s.replace('\\', "/")
        } else {
            s
        }
    });
    value
}
fn tree(root: &Path) -> Value {
    use base64::{Engine, engine::general_purpose::STANDARD};
    fn visit(root: &Path, current: &Path, result: &mut serde_json::Map<String, Value>) {
        for entry in fs::read_dir(current).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                visit(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace('\\', "/"),
                    json!(STANDARD.encode(fs::read(path).unwrap())),
                );
            }
        }
    }
    let mut files = serde_json::Map::new();
    visit(root, root, &mut files);
    Value::Object(files)
}
fn actual_pipeline(root: &Path, case: &Value, stamp: &str) -> Value {
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
        tree(&root.join("themes"))
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
    let originals = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/themes");
    let original_bytes = tree(&originals);
    let entries = fs::read_dir(&originals)
        .unwrap()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    assert_eq!(
        entries
            .iter()
            .filter(|e| e.file_type().unwrap().is_dir())
            .count(),
        4
    );
    assert_eq!(
        entries
            .iter()
            .filter(|e| e.file_type().unwrap().is_file()
                && e.path().extension().is_some_and(|x| x == "json"))
            .count(),
        7
    );
    assert!(!originals.join("exiles_corruption").exists());
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
        assert_eq!(
            tree(&originals),
            original_bytes,
            "bundled originals must not be written"
        );
    }
}

#[test]
fn actual_typescript_css_and_asset_delivery_edges() {
    use echosight_core::theme::ThemeRoot;
    let oracle = fixture();
    let temp = tempfile::tempdir().unwrap();
    write_files(&temp.path().join("asset-cases"), &oracle["assetFiles"]);
    let root = ThemeRoot::new(temp.path().join("asset-cases")).unwrap();
    for case in oracle["assetCases"].as_array().unwrap() {
        let mut theme = case["theme"].clone();
        map_strings(&mut theme, &|s| {
            s.replace("$ROOT", temp.path().to_str().unwrap())
        });
        let actual = if case["kind"] == "css" {
            json!(root.load_css(theme.as_object(), &case["name"]).unwrap())
        } else {
            json!(root.load_asset(theme.as_object(), &case["name"]).unwrap())
        };
        assert_eq!(actual, case["result"], "{case}");
    }
    for case in oracle["resolvedSounds"].as_array().unwrap() {
        let mut theme = case["theme"].clone();
        map_strings(&mut theme, &|s| {
            s.replace("$ROOT", temp.path().to_str().unwrap())
        });
        let sound = echosight_core::theme::sounds::resolve_sound(
            &root,
            theme.as_object(),
            case["event"].as_str().unwrap(),
            true,
            50.0,
        )
        .unwrap();
        let played = sound.map(|s| json!({"source":format!("data:{};base64,{}", s.asset["mimeType"].as_str().unwrap(), s.asset["data"].as_str().unwrap()),"volume":s.volume})).into_iter().collect::<Vec<_>>();
        assert_eq!(json!(played), case["played"]);
    }
}

#[test]
fn actual_typescript_nonfinite_volume_fallbacks_and_all_events() {
    use echosight_core::theme::sounds::{playback_volume, resolve_theme_sound_definition};
    for case in fixture()["soundVolumes"].as_array().unwrap() {
        let theme = case["theme"].as_object().unwrap();
        let def =
            resolve_theme_sound_definition(Some(theme), case["event"].as_str().unwrap()).unwrap();
        let percent: f64 = case["settingsPercent"].as_str().unwrap().parse().unwrap();
        assert_eq!(
            playback_volume(percent, theme, &def),
            case["volume"].as_f64().unwrap()
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
