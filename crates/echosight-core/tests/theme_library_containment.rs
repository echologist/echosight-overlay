//! Hardening tests are deliberately separate from TypeScript parity: the source
//! only checks lexical containment. No symlink may expose or overwrite outsiders.
use echosight_core::theme::{ThemeLibrary, ThemeRoot, sounds::resolve_sound};
use serde_json::json;
use std::{fs, path::Path};

fn write(path: &Path, bytes: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

#[test]
fn explicit_roots_and_lexical_containment() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("themes");
    fs::create_dir(&dir).unwrap();
    let root = ThemeRoot::new(&dir).unwrap();
    write(&temp.path().join("secret.css"), "outside");
    write(&temp.path().join("secret.wav"), "outside");
    let theme = json!({"folderPath":dir,"cssFile":"../secret.css","assets":{
        "outside":{"path":temp.path().join("secret.wav"),"type":"sound","extension":".wav"}
    }});
    assert!(
        root.load_css(theme.as_object(), &json!("../secret.css"))
            .unwrap()
            .is_none()
    );
    assert!(
        root.load_asset(theme.as_object(), &json!("outside"))
            .unwrap()
            .is_none()
    );
    let forged = json!({"folderPath":temp.path(),"cssFile":"secret.css"});
    assert!(
        root.load_css(forged.as_object(), &json!("secret.css"))
            .unwrap()
            .is_none()
    );
    assert!(root.existing(&dir.join("../secret.wav")).is_err());
    assert!(ThemeRoot::new(dir.join("missing")).is_err());
}

#[test]
fn missing_non_audio_and_disabled_sounds_do_not_resolve() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    write(
        &dir.join("theme.json"),
        r#"{"id":"s","name":"s","sounds":{"volume":0.8,"events":{"undo":"a.wav","redo":"missing.ogg","taskCompleted":"image.png","backgroundActivated":{"file":"a.wav","volume":0.5}}}}"#,
    );
    write(&dir.join("a.wav"), "audio");
    write(&dir.join("image.png"), "image");
    let mut library = ThemeLibrary::new(dir).unwrap();
    // The folder itself is a theme, so load it from its explicitly selected parent.
    let root = ThemeRoot::new(dir).unwrap();
    let theme = json!({"id":"s","name":"s","folderPath":dir,
        "sounds":{"volume":0.8,"events":{"undo":"a.wav","redo":"missing.ogg","taskCompleted":"image.png","backgroundActivated":{"file":"a.wav","volume":0.5}}},
        "assets":{"a":{"path":dir.join("a.wav"),"type":"sound","extension":".wav"},"image":{"path":dir.join("image.png"),"type":"misc","extension":".png"},"missing":{"path":dir.join("missing.ogg"),"type":"sound","extension":".ogg"}}});
    let t = theme.as_object();
    let played = resolve_sound(&root, t, "undo", true, 50.0)
        .unwrap()
        .unwrap();
    assert_eq!(played.volume, 0.4);
    assert_eq!(played.asset["data"], "YXVkaW8=");
    assert_eq!(
        resolve_sound(&root, t, "backgroundActivated", true, 50.0)
            .unwrap()
            .unwrap()
            .volume,
        0.2
    );
    for event in ["redo", "taskCompleted", "absent"] {
        assert!(
            resolve_sound(&root, t, event, true, 50.0)
                .unwrap()
                .is_none()
        );
    }
    assert!(
        resolve_sound(&root, t, "undo", false, 50.0)
            .unwrap()
            .is_none()
    );
    library.reload("fixed");
    fs::remove_file(dir.join("theme.json")).unwrap();
    library.reload("fixed");
    assert!(library.themes.is_empty());
}

#[cfg(unix)]
#[test]
fn an_explicitly_authorized_root_alias_retains_lexical_theme_paths() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let actual = temp.path().join("actual");
    let alias = temp.path().join("selected");
    write(
        &actual.join("theme.json"),
        r#"{"id":"s","name":"s","cssFile":"style.css"}"#,
    );
    write(&actual.join("style.css"), ".ok {}");
    write(
        &actual.join("folder/theme.json"),
        r#"{"id":"f","name":"f","variants":{"red":{}}}"#,
    );
    write(&actual.join("folder/a.wav"), "audio");
    write(&actual.join("folder/preview.png"), "image");
    write(&actual.join("folder/README.md"), "docs");
    write(&actual.join("folder/variants/red/a.wav"), "red audio");
    symlink(&actual, &alias).unwrap();
    let mut library = ThemeLibrary::new(&alias).unwrap();
    library.reload("fixed");
    let theme = library.get("s").unwrap();
    assert_eq!(theme["path"], json!(alias.join("theme.json")));
    assert_eq!(
        library
            .root()
            .load_css(Some(theme), &json!("style.css"))
            .unwrap(),
        Some(".ok {}".into())
    );
    let folder = library.get("f").unwrap();
    assert_eq!(folder["metadata"]["assetCount"], 2);
    assert_eq!(folder["documentation"]["content"], "docs");
    assert_eq!(
        library
            .root()
            .load_asset(library.get("f-red"), &json!("a"))
            .unwrap()
            .unwrap()["data"],
        "cmVkIGF1ZGlv"
    );
}

#[cfg(unix)]
#[test]
fn symlinks_cannot_read_outside_assets_css_metadata_or_variants() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("themes/folder");
    let outside = temp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    write(
        &dir.join("theme.json"),
        r#"{"id":"s","name":"s","cssFile":"style.css","variants":{"red":{}}}"#,
    );
    for file in ["style.css", "a.wav", "preview.png", "README.md"] {
        write(&outside.join(file), "secret");
        symlink(outside.join(file), dir.join(file)).unwrap();
    }
    fs::create_dir(dir.join("variants")).unwrap();
    symlink(&outside, dir.join("variants/red")).unwrap();
    let mut library = ThemeLibrary::new(temp.path().join("themes")).unwrap();
    library.reload("fixed");
    let theme = library.get("s").unwrap();
    assert_eq!(theme["assets"], json!({}));
    assert!(!theme.contains_key("preview"));
    assert!(!theme.contains_key("documentation"));
    assert_eq!(library.get("s-red").unwrap()["assets"], json!({}));
    assert!(
        library
            .root()
            .load_css(Some(theme), &json!("style.css"))
            .unwrap()
            .is_none()
    );
    let forged = json!({"folderPath":dir,"assets":{"a":{"path":dir.join("a.wav"),"type":"sound","extension":".wav"}}});
    assert!(
        library
            .root()
            .load_asset(forged.as_object(), &json!("a"))
            .unwrap()
            .is_none()
    );
}

#[cfg(unix)]
fn assert_sync_rejects_retargeted_root(file: &str, existing: bool, initial_alias: bool) {
    use echosight_core::theme::sync::sync_bundled;
    use std::{io::ErrorKind, os::unix::fs::symlink};
    let temp = tempfile::tempdir().unwrap();
    let bundled = temp.path().join("bundled");
    let authorized = temp.path().join("authorized");
    let selected = temp.path().join("selected");
    let outside = temp.path().join("outside");
    fs::create_dir(&authorized).unwrap();
    fs::create_dir(&outside).unwrap();
    write(&bundled.join(file), r#"{"id":"s","version":"2"}"#);
    write(&outside.join("sentinel"), "untouched");
    let old = r#"{"id":"s","version":"1"}"#;
    if existing {
        write(&outside.join(file), old);
    }
    if initial_alias {
        symlink(&authorized, &selected).unwrap();
    } else {
        fs::create_dir(&selected).unwrap();
    }
    let source = ThemeRoot::new(&bundled).unwrap();
    let target = ThemeRoot::new(&selected).unwrap();
    // Sequential replacement after authorization, before sync: no race needed.
    if initial_alias {
        fs::remove_file(&selected).unwrap();
    } else {
        fs::remove_dir(&selected).unwrap();
    }
    symlink(&outside, &selected).unwrap();
    assert_eq!(
        target.existing(&selected).unwrap_err().kind(),
        ErrorKind::PermissionDenied
    );
    let result = sync_bundled(&source, &target);
    // Check side effects before the return value, so RED proves the actual escape.
    assert_eq!(
        fs::read_to_string(outside.join("sentinel")).unwrap(),
        "untouched"
    );
    if existing {
        assert_eq!(fs::read_to_string(outside.join(file)).unwrap(), old);
    } else {
        let first = Path::new(file).components().next().unwrap();
        assert!(
            !outside.join(first).exists(),
            "sync wrote outside the authorized root: {file}; result: {result:?}"
        );
    }
    assert_eq!(fs::read_dir(&authorized).unwrap().count(), 0);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::PermissionDenied);
}

#[cfg(unix)]
#[test]
fn sync_rejects_retargeted_root_before_missing_file_write() {
    assert_sync_rejects_retargeted_root("new.json", false, true);
}

#[cfg(unix)]
#[test]
fn sync_rejects_retargeted_root_before_missing_nested_directory_creation() {
    assert_sync_rejects_retargeted_root("folder/nested/new.json", false, true);
}

#[cfg(unix)]
#[test]
fn sync_rejects_retargeted_root_with_existing_destination() {
    assert_sync_rejects_retargeted_root("theme.json", true, true);
}

#[cfg(unix)]
#[test]
fn sync_rejects_authorized_directory_replaced_by_symlink() {
    assert_sync_rejects_retargeted_root("new.json", false, false);
}

#[cfg(unix)]
#[test]
fn sync_rejects_root_alias_retargeted_to_authorized_descendant() {
    use echosight_core::theme::sync::sync_bundled;
    use std::{io::ErrorKind, os::unix::fs::symlink};
    let temp = tempfile::tempdir().unwrap();
    let bundled = temp.path().join("bundled");
    let authorized = temp.path().join("authorized");
    let child = authorized.join("child");
    let selected = temp.path().join("selected");
    fs::create_dir_all(&child).unwrap();
    write(&bundled.join("new.json"), "bundled");
    symlink(&authorized, &selected).unwrap();
    let source = ThemeRoot::new(&bundled).unwrap();
    let target = ThemeRoot::new(&selected).unwrap();
    fs::remove_file(&selected).unwrap();
    symlink(&child, &selected).unwrap();
    // Still contained, but it is no longer the root that was authorized.
    assert_eq!(
        target.existing(&selected).unwrap(),
        fs::canonicalize(&child).unwrap()
    );
    let result = sync_bundled(&source, &target);
    assert!(!child.join("new.json").exists());
    assert_eq!(result.unwrap_err().kind(), ErrorKind::PermissionDenied);
}

#[cfg(unix)]
#[test]
fn sync_allows_stable_authorized_root_alias() {
    use echosight_core::theme::sync::sync_bundled;
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let bundled = temp.path().join("bundled");
    let authorized = temp.path().join("authorized");
    let selected = temp.path().join("selected");
    fs::create_dir(&authorized).unwrap();
    let new = r#"{"id":"s","version":"2"}"#;
    write(&bundled.join("theme.json"), new);
    write(&bundled.join("folder/nested/a.wav"), "audio");
    write(
        &authorized.join("theme.json"),
        r#"{"id":"s","version":"1"}"#,
    );
    symlink(&authorized, &selected).unwrap();
    let source = ThemeRoot::new(&bundled).unwrap();
    let target = ThemeRoot::new(&selected).unwrap();
    sync_bundled(&source, &target).unwrap();
    assert_eq!(
        fs::read_to_string(authorized.join("theme.json")).unwrap(),
        new
    );
    assert_eq!(
        fs::read_to_string(authorized.join("folder/nested/a.wav")).unwrap(),
        "audio"
    );
    assert!(
        fs::symlink_metadata(&selected)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(target.path(), selected);
}

#[cfg(unix)]
#[test]
fn sync_rejects_destination_and_source_symlinks_without_outside_writes() {
    use echosight_core::theme::sync::sync_bundled;
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let bundled = temp.path().join("bundled");
    let target = temp.path().join("target");
    let outside = temp.path().join("outside");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(&outside).unwrap();
    write(&bundled.join("theme.json"), r#"{"id":"s","version":"2"}"#);
    write(&outside.join("theme.json"), r#"{"id":"s","version":"1"}"#);
    let before = fs::read(outside.join("theme.json")).unwrap();
    symlink(outside.join("theme.json"), target.join("theme.json")).unwrap();
    let source = ThemeRoot::new(&bundled).unwrap();
    let destination = ThemeRoot::new(&target).unwrap();
    assert!(sync_bundled(&source, &destination).is_err());
    assert_eq!(fs::read(outside.join("theme.json")).unwrap(), before);
    fs::remove_file(target.join("theme.json")).unwrap();
    symlink(outside.join("missing.json"), target.join("theme.json")).unwrap();
    assert!(sync_bundled(&source, &destination).is_err());
    assert!(!outside.join("missing.json").exists());
    fs::remove_file(target.join("theme.json")).unwrap();
    fs::remove_file(bundled.join("theme.json")).unwrap();
    write(
        &bundled.join("folder/theme.json"),
        r#"{"id":"f","version":"2"}"#,
    );
    symlink(&outside, bundled.join("folder/linked")).unwrap();
    assert!(sync_bundled(&source, &destination).is_err());
    assert!(!target.join("folder/linked/theme.json").exists());
    assert_eq!(fs::read(outside.join("theme.json")).unwrap(), before);
    fs::remove_file(bundled.join("folder/linked")).unwrap();
    write(&bundled.join("folder/linked/a.wav"), "bundled");
    write(
        &target.join("folder/theme.json"),
        r#"{"id":"f","version":"1"}"#,
    );
    symlink(&outside, target.join("folder/linked")).unwrap();
    assert!(sync_bundled(&source, &destination).is_err());
    assert!(!outside.join("a.wav").exists());
}
