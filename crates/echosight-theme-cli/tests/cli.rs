use echosight_core::theme::v2::Theme;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Sandbox(PathBuf);
impl Sandbox {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "echosight-cli-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, name: &str, value: &Value) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
        path
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_echosight-theme"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap()
    }
    fn json(&self, args: &[&str], success: bool) -> Value {
        let out = self.run(args);
        assert_eq!(
            out.status.success(),
            success,
            "args={args:?}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).expect("structured stdout even on failure")
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn v1() -> Value {
    json!({"id":"scratch-custom","name":"Custom","colors":{"text":{"primary":"#123456"}}})
}
fn tree(path: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                walk(root, &entry.path(), out);
            } else {
                out.insert(
                    entry.path().strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(entry.path()).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(path, path, &mut out);
    out
}

#[test]
fn validates_formats_errors_and_does_not_read_v2_css() {
    let s = Sandbox::new();
    s.write("v1.json", &v1());
    let r = s.json(&["validate", "v1.json", "--json"], true);
    assert_eq!(r["format"], 1);
    s.write(
        "v2.json",
        &json!({"format":2,"id":"two","name":"Two","cssFile":"/missing.css"}),
    );
    let r = s.json(&["--json", "validate", "v2.json"], false);
    assert_eq!(r["diagnostics"][0]["code"], "forbidden");
    assert!(!r["diagnostics"].as_array().unwrap().is_empty());
    for (name, doc) in [
        (
            "future.json",
            json!({"format":3,"id":"future","name":"Future"}),
        ),
        (
            "bad.json",
            json!({"format":2,"id":"bad","name":"Bad","slots":{"button":{"fill":{"color":"@missing"}}}}),
        ),
        (
            "union.json",
            json!({"format":2,"id":"union","name":"Union","slots":{"button":{"fill":{"color":"@missing","stops":[{"color":"#000","offset":0},{"color":"#fff","offset":1}]}}}}),
        ),
    ] {
        s.write(name, &doc);
        let r = s.json(&["validate", name, "--json"], false);
        assert_eq!(r["ok"], false);
    }
    fs::write(s.0.join("malformed.json"), "{").unwrap();
    for name in ["malformed.json", "absent.json"] {
        s.json(&["validate", name, "--json"], false);
    }
    assert!(String::from_utf8_lossy(&s.run(&["doctor"]).stderr).contains("not implemented"));
}

#[test]
fn source_driven_conversion_reports_losses_and_refuses_existing_output() {
    let s = Sandbox::new();
    let mut source = v1();
    source["customCSS"] = json!(".unmapped { transform: rotate(12deg); }");
    s.write("source/theme.json", &source);
    let before = tree(&s.0.join("source"));
    let r = s.json(&["convert", "source", "out", "--json"], true);
    assert_eq!(r["ok"], true);
    assert!(!r["diagnostics"].as_array().unwrap().is_empty());
    let converted: Value =
        serde_json::from_slice(&fs::read(s.0.join("out/theme.json")).unwrap()).unwrap();
    assert_eq!(converted["id"], "scratch-custom");
    assert_eq!(converted["tokens"]["colors"]["text"]["primary"], "#123456");
    Theme::parse(&converted).unwrap();
    assert!(
        fs::read_to_string(s.0.join("out/CONVERSION-REPORT.md"))
            .unwrap()
            .contains("unsupported-selector")
    );
    let output = tree(&s.0.join("out"));
    s.json(&["convert", "source/theme.json", "out", "--json"], false);
    assert_eq!(output, tree(&s.0.join("out")));
    fs::write(s.0.join("exists"), "keep").unwrap();
    s.json(&["convert", "source", "exists", "--json"], false);
    s.json(&["convert", "source", "source/inside", "--json"], false);
    assert_eq!(before, tree(&s.0.join("source")));
    s.json(&["convert", "source/theme.json", "second", "--json"], true);
    assert_eq!(output, tree(&s.0.join("second")));
}

#[test]
fn converter_budget_failure_never_creates_output() {
    let s = Sandbox::new();
    let mut doc = v1();
    for i in 0..530 {
        doc[format!("unmapped{i}")] = json!(i);
    }
    s.write("src/theme.json", &doc);
    let r = s.json(&["convert", "src", "out", "--json"], false);
    assert!(
        r["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "budget")
    );
    assert!(!s.0.join("out").exists());
    assert_eq!(fs::read_dir(&s.0).unwrap().count(), 1);
}

#[test]
fn mixed_list_is_deterministic_readonly_with_variants_before_base() {
    let s = Sandbox::new();
    let mut doc = v1();
    doc["variants"] = json!({"night":{"name":"Night"}});
    s.write("themes/a/theme.json", &doc);
    s.write(
        "themes/b.json",
        &json!({"format":2,"id":"two","name":"Two","variants":{"light":{"name":"Light"}}}),
    );
    s.write("themes/ignored.JSON", &json!({"format":99}));
    let before = tree(&s.0.join("themes"));
    let a = s.json(&["list", "themes", "--json"], true);
    let b = s.json(&["list", "themes", "--json"], true);
    assert_eq!(a, b);
    let ids: Vec<_> = a["themes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        ["scratch-custom-night", "scratch-custom", "two-light", "two"]
    );
    assert_eq!(before, tree(&s.0.join("themes")));
    s.write("themes/c.json", &json!({"format":7}));
    let r = s.json(&["list", "themes", "--json"], false);
    assert_eq!(r["themes"].as_array().unwrap().len(), 4);
}

#[test]
fn variant_folder_overrides_fail_explicitly_instead_of_inheriting_wrong_assets() {
    let s = Sandbox::new();
    let mut doc = v1();
    doc["variants"] = json!({"night":{"name":"Night"}});
    s.write("src/theme.json", &doc);
    fs::create_dir_all(s.0.join("src/variants/night")).unwrap();
    fs::write(s.0.join("src/main_bg.svg"), "base").unwrap();
    fs::write(s.0.join("src/variants/night/main_bg.svg"), "override").unwrap();
    let r = s.json(&["convert", "src", "out", "--json"], false);
    assert!(
        r["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "variant-assets")
    );
    assert!(!s.0.join("out").exists());
}

#[test]
fn standalone_json_converts_to_new_sibling_from_unrelated_directory() {
    let s = Sandbox::new();
    s.write("custom.json", &v1());
    s.json(&["convert", "custom.json", "out", "--json"], true);
    s.json(&["validate", "out", "--json"], true);
}

#[test]
fn readonly_validation_warnings_are_not_conversion_losses() {
    let s = Sandbox::new();
    let mut doc = v1();
    doc["customCSS"] = json!(".unsupported { transform: rotate(2deg); }");
    s.write("source/theme.json", &doc);
    let before = tree(&s.0);
    let checked = s.json(&["validate", "source", "--json"], true);
    assert!(checked["diagnostics"].as_array().unwrap().is_empty());
    assert_eq!(before, tree(&s.0));
    let converted = s.json(&["convert", "source", "out", "--json"], true);
    assert!(!converted["diagnostics"].as_array().unwrap().is_empty());
    s.json(&["convert", "out", "downgrade", "--json"], false);
    assert!(!s.0.join("downgrade").exists());
    let human = s.run(&["convert", "source", "human"]);
    assert!(human.status.success());
    assert!(String::from_utf8_lossy(&human.stderr).contains("unsupported-selector"));
}

#[test]
fn bounded_read_and_listing_fail_explicitly_without_writes() {
    let s = Sandbox::new();
    fs::write(s.0.join("huge.json"), vec![b' '; 1024 * 1024 + 1]).unwrap();
    let r = s.json(&["validate", "huge.json", "--json"], false);
    assert_eq!(r["diagnostics"][0]["code"], "budget");
    fs::create_dir(s.0.join("themes")).unwrap();
    for i in 0..1025 {
        fs::write(s.0.join(format!("themes/ignored-{i}")), "").unwrap();
    }
    let r = s.json(&["list", "themes", "--json"], false);
    assert_eq!(r["diagnostics"][0]["code"], "budget");
    s.json(&["list", "absent", "--json"], false);
    assert!(!s.0.join("absent").exists());
}

#[test]
fn all_eleven_frozen_sources_convert_with_real_assets_and_exact_reports() {
    let s = Sandbox::new();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixtures = repo.join("crates/echosight-core/tests/theme_v2_fixtures");
    let manifest: Value =
        serde_json::from_slice(&fs::read(fixtures.join("manifest.json")).unwrap()).unwrap();
    let themes = manifest["themes"].as_array().unwrap();
    assert_eq!(themes.len(), 11);
    for entry in themes {
        let id = entry["id"].as_str().unwrap();
        let source_dir = s.0.join(format!("source-{id}"));
        fs::create_dir(&source_dir).unwrap();
        let frozen = fixtures.join(id);
        fs::copy(frozen.join("source.json"), source_dir.join("theme.json")).unwrap();
        let original = repo
            .join("data/themes")
            .join(entry["source"].as_str().unwrap());
        if let Some(css) = entry["css"].as_str() {
            let name = Path::new(css).file_name().unwrap();
            fs::copy(frozen.join("source.css"), source_dir.join(name)).unwrap();
            for asset in entry["assets"].as_array().unwrap() {
                let asset = asset.as_str().unwrap();
                fs::copy(
                    original.parent().unwrap().join(asset),
                    source_dir.join(asset),
                )
                .unwrap();
            }
        }
        let before = tree(&source_dir);
        let input = format!("source-{id}");
        let output = format!("out-{id}");
        let r = s.json(&["convert", &input, &output, "--json"], true);
        let expected: Value =
            serde_json::from_slice(&fs::read(frozen.join("converted.json")).unwrap()).unwrap();
        let actual: Value =
            serde_json::from_slice(&fs::read(s.0.join(&output).join("theme.json")).unwrap())
                .unwrap();
        assert_eq!(actual, expected["theme"], "{id}");
        assert_eq!(r["diagnostics"], expected["diagnostics"], "{id}");
        assert_eq!(
            fs::read(s.0.join(&output).join("CONVERSION-REPORT.md")).unwrap(),
            fs::read(frozen.join("CONVERSION-REPORT.md")).unwrap(),
            "{id}"
        );
        let parsed = Theme::parse(&actual).unwrap();
        for slot in parsed.slot_names() {
            assert!(parsed.slot(slot, &[], 1.).is_some());
        }
        for (path, bytes) in tree(&s.0.join(&output)) {
            if path != Path::new("theme.json") && path != Path::new("CONVERSION-REPORT.md") {
                assert_eq!(&bytes, before.get(&path).unwrap());
            }
        }
        if id == "winamp-classic" {
            assert_eq!(
                fs::read(s.0.join(&output).join("complete.wav")).unwrap(),
                before[Path::new("complete.wav")]
            );
            assert!(s.0.join(&output).join("main_bg.svg").exists());
        }
        assert_eq!(before, tree(&source_dir));
        s.json(&["validate", &input, "--json"], true);
    }
    let before = tree(&s.0);
    // Both v1 sources and v2 outputs: duplicate IDs replace their original slot.
    let listed = s.json(&["list", ".", "--json"], true);
    assert_eq!(listed["themes"].as_array().unwrap().len(), 11);
    assert_eq!(before, tree(&s.0));
}

#[test]
fn exact_sound_assets_and_variant_inheritance() {
    let s = Sandbox::new();
    let mut doc = v1();
    doc["sounds"] = json!({"events":{"taskCompleted":{"file":"Complete.WAV"}}});
    doc["variants"] = json!({"night":{"name":"Night"}});
    s.write("source/theme.json", &doc);
    fs::create_dir(s.0.join("source/nested")).unwrap();
    // Nested CSS is authorized explicitly; nested audio is rejected by v1 core.
    doc["cssFile"] = json!("nested/style.css");
    s.write("source/theme.json", &doc);
    fs::write(s.0.join("source/nested/style.css"), "").unwrap();
    fs::write(s.0.join("source/Complete.WAV"), b"sound bytes").unwrap();
    fs::write(s.0.join("source/main_bg.svg"), b"<svg/>").unwrap();
    fs::write(s.0.join("source/unused.svg"), b"unused").unwrap();
    s.json(&["convert", "source", "out", "--json"], true);
    assert_eq!(
        fs::read(s.0.join("out/Complete.WAV")).unwrap(),
        b"sound bytes"
    );
    assert!(!s.0.join("out/unused.svg").exists());
    let actual: Value =
        serde_json::from_slice(&fs::read(s.0.join("out/theme.json")).unwrap()).unwrap();
    let t = Theme::parse(&actual).unwrap();
    let v = t.variant("night").unwrap();
    assert_eq!(
        v.document()["sounds"]["events"]["taskCompleted"]["file"],
        "Complete.WAV"
    );
    assert_eq!(
        v.document()["images"]["window.interactive"]["asset"],
        "main_bg.svg"
    );
}

#[test]
fn missing_css_or_asset_and_unsafe_declared_paths_fail_without_output() {
    let s = Sandbox::new();
    for (i, path) in [
        "missing.css",
        "../escape.css",
        "/absolute.css",
        "C:\\absolute.css",
    ]
    .iter()
    .enumerate()
    {
        let mut doc = v1();
        doc["cssFile"] = json!(path);
        s.write("src/theme.json", &doc);
        let out = format!("out{i}");
        s.json(&["convert", "src", &out, "--json"], false);
        assert!(!s.0.join(out).exists());
    }
    let mut doc = v1();
    doc["sounds"] = json!({"events":{"taskCompleted":{"file":"absent.wav"}}});
    s.write("src/theme.json", &doc);
    s.json(&["convert", "src", "missing-asset", "--json"], false);
    assert!(!s.0.join("missing-asset").exists());
}

#[cfg(unix)]
#[test]
fn symlink_escapes_and_dangling_output_are_rejected_stable_alias_is_allowed() {
    use std::os::unix::fs::symlink;
    let s = Sandbox::new();
    let mut doc = v1();
    doc["cssFile"] = json!("style.css");
    s.write("src/theme.json", &doc);
    fs::write(s.0.join("outside.css"), "body {}").unwrap();
    symlink("../outside.css", s.0.join("src/style.css")).unwrap();
    s.json(&["convert", "src", "out", "--json"], false);
    assert!(!s.0.join("out").exists());
    fs::remove_file(s.0.join("src/style.css")).unwrap();
    fs::write(s.0.join("src/style.css"), "").unwrap();
    symlink("nowhere", s.0.join("out")).unwrap();
    s.json(&["convert", "src", "out", "--json"], false);
    assert!(
        fs::symlink_metadata(s.0.join("out"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    symlink("src", s.0.join("alias")).unwrap();
    s.json(&["convert", "alias", "safe", "--json"], true);
    symlink("../outside.css", s.0.join("src/main_bg.svg")).unwrap();
    s.json(&["convert", "src", "unsafe-asset", "--json"], false);
}
