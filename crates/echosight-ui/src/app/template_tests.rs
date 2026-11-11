use super::*;
use gpui::{TestAppContext, VisualTestContext, px, size};
use serde_json::json;

fn click(cx: &mut VisualTestContext, selector: &'static str) {
    let bounds = cx
        .debug_bounds(selector)
        .unwrap_or_else(|| panic!("missing {selector}"));
    cx.simulate_click(bounds.center(), Default::default());
    cx.run_until_parked();
}
fn open<'a>(
    cx: &'a mut TestAppContext,
    dir: &std::path::Path,
) -> (Entity<TaskApp>, &'a mut VisualTestContext) {
    cx.update(crate::components::text_input::init);
    let (scene, cx) = cx.add_window_view(|w, cx| TaskApp::new(dir.into(), w, cx));
    cx.simulate_resize(size(px(500.), px(700.)));
    cx.run_until_parked();
    (scene, cx)
}
fn disk(dir: &std::path::Path, file: &str) -> Value {
    serde_json::from_slice(&std::fs::read(dir.join(file)).unwrap()).unwrap()
}
fn seed(dir: &std::path::Path) {
    std::fs::write(dir.join("templates.json"), "[]").unwrap();
    std::fs::write(dir.join("current_tasks.json"), json!({"tasks":[
        {"id":1,"text":"parent","completed":false,"createdAt":"x","children":[
            {"id":2,"text":"child","completed":false,"createdAt":"x","triggers":[4]},
            {"id":3,"text":"done","completed":true,"createdAt":"x"}]},
        {"id":4,"text":"background","completed":false,"createdAt":"x","mode":"background","activated":false,
         "backgroundOptions":{"priority":"high","expiresAfterMinutes":1}}
    ],"currentTemplate":null,"snapshots":[]}).to_string()).unwrap();
}
struct RecordingPorts {
    save: Result<Option<PathBuf>, String>,
    names: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}
impl ports::TemplatePorts for RecordingPorts {
    fn prompt_open_file(&self, _: &mut gpui::App) -> ports::PathPrompt {
        Box::pin(async { Err("picker unavailable".into()) })
    }
    fn prompt_save_file(
        &self,
        _: &std::path::Path,
        name: &str,
        _: &mut gpui::App,
    ) -> ports::PathPrompt {
        self.names.borrow_mut().push(name.into());
        let result = self.save.clone();
        Box::pin(async move { result })
    }
}
#[gpui::test]
fn template_injected_prompt_filename_failure_and_cancel_are_non_destructive(
    cx: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let (scene, vc) = open(cx, dir.path());
    click(vc, "template-save");
    vc.simulate_keystrokes("a enter");
    vc.run_until_parked();
    click(vc, "template-choose");
    click(vc, "template-option-0");
    let names = std::rc::Rc::new(std::cell::RefCell::new(vec![]));
    for outcome in [
        Ok(None),
        Err("save dialog failed".into()),
        Ok(Some(dir.path().to_path_buf())),
    ] {
        scene.update(vc, |s, _| {
            s.templates.ports = Some(std::rc::Rc::new(RecordingPorts {
                save: outcome,
                names: names.clone(),
            }))
        });
        click(vc, "template-export");
        assert_eq!(
            disk(dir.path(), "templates.json").as_array().unwrap().len(),
            1
        );
    }
    assert_eq!(*names.borrow(), vec!["poe2-template-a.json"; 3]);
    assert!(scene.read_with(vc, |s, _| s.templates.error.is_some()));
    click(vc, "template-import");
    click(vc, "template-file");
    assert!(scene.read_with(vc, |s, _| {
        s.form_error
            .as_ref()
            .unwrap()
            .contains("picker unavailable")
    }));
}
#[gpui::test]
fn template_preload_guard_and_shared_initialization_error(cx: &mut TestAppContext) {
    cx.update(crate::components::text_input::init);
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    let (_, vc) = cx.add_window_view(|w, cx| {
        let mut app = TaskApp::new(missing.clone(), w, cx);
        assert!(app.templates.collection.is_none());
        app.activate(Command::Template(TemplateCommand::Save), w, cx);
        app.activate(Command::Template(TemplateCommand::Community), w, cx);
        assert!(app.dialog.is_none());
        app
    });
    vc.run_until_parked();
    assert!(missing.join("settings.json").exists());
    assert!(!missing.join("current_tasks.json").exists());
    assert!(!missing.join("templates.json").exists());
    let (scene, vc) = cx.add_window_view(|w, cx| {
        TaskApp::with_storage(Err("invalid authorized path".into()), w, cx)
    });
    vc.run_until_parked();
    scene.read_with(vc, |s, _| {
        assert!(s.templates.error.is_some() && s.error.is_some())
    });
}
#[gpui::test]
fn template_pending_native_export_blocks_quit_until_prompt_resolves(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let (scene, vc) = open(cx, dir.path());
    click(vc, "template-save");
    vc.simulate_keystrokes("a enter");
    vc.run_until_parked();
    click(vc, "template-choose");
    click(vc, "template-option-0");
    click(vc, "template-export");
    vc.simulate_close();
    vc.run_until_parked();
    scene.read_with(vc, |s, _| {
        assert!(
            !s.tab_order
                .iter()
                .any(|(_, command)| matches!(command, Some(Command::Accept)))
        )
    });
    vc.simulate_new_path_selection(|_| None);
    vc.run_until_parked();
    scene.read_with(vc, |s, _| {
        assert!(
            s.tab_order
                .iter()
                .any(|(_, command)| matches!(command, Some(Command::Accept)))
        )
    });
}

#[gpui::test]
fn template_import_paste_inserts_at_selection_and_enter_does_not_submit(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let (scene, vc) = open(cx, dir.path());
    click(vc, "template-import");
    vc.write_to_clipboard(gpui::ClipboardItem::new_string(
        "{\"name\":\"p\",\"tasks\":[{\"text\":\"one\"}]".into(),
    ));
    vc.simulate_keystrokes("ctrl-v");
    vc.run_until_parked();
    vc.write_to_clipboard(gpui::ClipboardItem::new_string("}".into()));
    vc.simulate_keystrokes("ctrl-v");
    vc.run_until_parked();
    assert!(scene.read_with(vc, |s, cx| {
        s.editor.read(cx).text().starts_with("{\"name\"")
    }));
    vc.simulate_keystrokes("enter");
    vc.run_until_parked();
    assert!(scene.read_with(vc, |s, _| matches!(
        s.dialog,
        Some(Dialog::Template(TemplateDialog::Import))
    )));
    assert_eq!(disk(dir.path(), "templates.json"), json!([]));
    click(vc, "accept");
    assert_eq!(disk(dir.path(), "templates.json")[0]["name"], "p");
}

#[gpui::test]
fn template_selection_preserves_zero_and_duplicate_id_source_semantics(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    std::fs::write(
        dir.path().join("templates.json"),
        json!([
            {"id":0,"name":"zero","tasks":[{"text":"zero"}],"createdAt":"x"},
            {"id":10,"name":"first","tasks":[{"text":"first"}],"createdAt":"x"},
            {"id":10,"name":"second","tasks":[{"text":"second"}],"createdAt":"x"}
        ])
        .to_string(),
    )
    .unwrap();
    let (scene, vc) = open(cx, dir.path());
    click(vc, "template-choose");
    click(vc, "template-option-0");
    click(vc, "template-load");
    assert!(
        scene.read_with(vc, |s, _| s.dialog.is_none()),
        "source !templateId ignores zero"
    );
    click(vc, "template-choose");
    click(vc, "template-option-2");
    click(vc, "template-load");
    click(vc, "accept");
    assert_eq!(
        disk(dir.path(), "current_tasks.json")["currentTemplate"],
        "first"
    );
}

#[gpui::test]
fn template_load_gate_failures_keep_tasks_independent(cx: &mut TestAppContext) {
    for bytes in [
        "undefined".to_owned(),
        json!([{"name":"bad","tasks":[{"text":"x","triggerIndices":[{"toString":null}]}]}])
            .to_string(),
    ] {
        let dir = tempfile::tempdir().unwrap();
        seed(dir.path());
        std::fs::write(dir.path().join("templates.json"), &bytes).unwrap();
        let (scene, vc) = open(cx, dir.path());
        scene.read_with(vc, |s, _| {
            assert!(s.templates.collection.is_none());
            assert!(s.templates.error.is_some());
            assert!(s.writable());
        });
        click(vc, "template-save");
        click(vc, "template-import");
        assert!(scene.read_with(vc, |s, _| s.dialog.is_none()));
        vc.simulate_keystrokes("n e w enter");
        vc.run_until_parked();
        assert_eq!(
            disk(dir.path(), "current_tasks.json")["tasks"][2]["text"],
            "new"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("templates.json")).unwrap(),
            bytes
        );
    }
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    let (scene, vc) = open(cx, &missing);
    assert!(scene.read_with(vc, |s, _| s.templates.collection.is_some()));
    assert!(missing.join("settings.json").exists());
    assert!(!missing.join("current_tasks.json").exists());
    assert!(!missing.join("templates.json").exists());
}

#[derive(Default)]
struct TemplateProbe {
    writes: std::sync::Mutex<Vec<Value>>,
    fail: std::sync::atomic::AtomicBool,
}
impl echosight_core::storage::StorageHooks for TemplateProbe {
    fn before(
        &self,
        stage: echosight_core::storage::Stage,
        path: &std::path::Path,
    ) -> std::io::Result<()> {
        use echosight_core::storage::Stage;
        if stage == Stage::Sync
            && path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with(".templates.json."))
        {
            self.writes
                .lock()
                .unwrap()
                .push(serde_json::from_slice(&std::fs::read(path)?).unwrap());
        }
        if stage == Stage::Replace
            && path.file_name().is_some_and(|n| n == "templates.json")
            && self.fail.load(std::sync::atomic::Ordering::SeqCst)
        {
            return Err(std::io::Error::other("injected template replace failure"));
        }
        Ok(())
    }
}
#[gpui::test]
fn template_failed_save_retains_previous_collection_retry_fifo_and_error_visible(
    cx: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let probe = std::sync::Arc::new(TemplateProbe::default());
    cx.update(crate::components::text_input::init);
    let paths = DataPaths::new(dir.path()).unwrap();
    let storage = Storage::with_hooks(paths.clone(), probe.clone());
    let (scene, vc) =
        cx.add_window_view(|w, cx| TaskApp::with_storage(Ok((storage, paths)), w, cx));
    vc.simulate_resize(size(px(500.), px(700.)));
    vc.run_until_parked();
    assert!(probe.writes.lock().unwrap().is_empty());
    probe.fail.store(true, std::sync::atomic::Ordering::SeqCst);
    click(vc, "template-save");
    vc.simulate_keystrokes("a enter");
    vc.run_until_parked();
    assert_eq!(disk(dir.path(), "templates.json"), json!([]));
    scene.read_with(vc, |s, _| {
        assert!(s.templates.collection.as_ref().unwrap().is_empty());
        assert!(s.templates.error.is_some());
        assert!(s.error.is_none());
    });
    assert!(
        vc.debug_bounds("form-error").is_some(),
        "template failure must remain visible inside the open modal"
    );
    probe.fail.store(false, std::sync::atomic::Ordering::SeqCst);
    click(vc, "accept");
    click(vc, "template-save");
    vc.simulate_keystrokes("b enter");
    vc.run_until_parked();
    let writes = probe.writes.lock().unwrap();
    assert_eq!(
        writes
            .iter()
            .map(|v| v.as_array().unwrap().len())
            .collect::<Vec<_>>(),
        vec![1, 1, 2]
    );
    assert_eq!(disk(dir.path(), "templates.json"), writes[2]);
}
#[gpui::test]
fn template_community_modal_expiry_nested_trap_and_footer(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let mut tasks = disk(dir.path(), "current_tasks.json");
    tasks["tasks"][1]["activated"] = true.into();
    tasks["tasks"][1]["activatedAt"] = "2027-01-15T08:00:00.000Z".into();
    std::fs::write(dir.path().join("current_tasks.json"), tasks.to_string()).unwrap();
    cx.update(crate::components::text_input::init);
    let (scene, vc) = cx.add_window_view(|w, cx| {
        let clock =
            TaskClock::anchored(cx.background_executor().clone(), 1_800_000_000_000, || {
                0.017
            });
        TaskApp::new_with_runtime(dir.path().into(), clock, std::sync::Arc::new(|_| {}), w, cx)
    });
    vc.simulate_resize(size(px(450.), px(600.)));
    vc.run_until_parked();
    click(vc, "template-community");
    click(vc, "community-preview-0");
    let before = vc.debug_bounds("cancel").unwrap();
    vc.background_executor
        .advance_clock(std::time::Duration::from_secs(60));
    vc.run_until_parked();
    assert!(!scene.read_with(vc, |s, _| s.state().unwrap().tasks()[1].activated));
    assert_eq!(before, vc.debug_bounds("cancel").unwrap());
    vc.simulate_keystrokes("escape");
    vc.run_until_parked();
    scene.update_in(vc, |s, w, _| {
        assert!(s.controls["community-preview-0"].is_focused(w))
    });
    for _ in 0..20 {
        vc.simulate_keystrokes("tab");
    }
    scene.update_in(vc, |s, w, _| {
        assert!(s.tab_order.iter().any(|(h, _)| h.is_focused(w)))
    });
    let cancel = vc.debug_bounds("cancel").unwrap();
    assert!(cancel.bottom() <= px(600.) && cancel.top() >= px(0.));
    vc.simulate_keystrokes("escape");
    vc.run_until_parked();
    scene.update_in(vc, |s, w, _| {
        assert!(s.controls["template-community"].is_focused(w))
    });
}

#[gpui::test]
fn template_import_clipboard_replacement_nested_cancel_and_core_limits(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let (scene, vc) = open(cx, dir.path());
    let payload = "{\n\"name\":\"Imported\",\"tasks\":[{\"text\":\"one\"}]}";
    for pass in 0..2 {
        click(vc, "template-import");
        vc.write_to_clipboard(gpui::ClipboardItem::new_string(payload.into()));
        vc.simulate_keystrokes("ctrl-v");
        vc.run_until_parked();
        click(vc, "accept");
        if pass == 1 {
            assert!(scene.read_with(vc, |s, _| s.dialog.is_some()));
            let before = disk(dir.path(), "templates.json");
            vc.simulate_keystrokes("escape");
            vc.run_until_parked();
            assert!(scene.read_with(vc, |s, _| s.dialog.is_some()));
            assert_eq!(disk(dir.path(), "templates.json"), before);
            click(vc, "accept");
            click(vc, "accept");
        }
        assert!(scene.read_with(vc, |s, _| s.dialog.is_none()));
        assert_eq!(disk(dir.path(), "templates.json")[0]["imported"], true);
    }
    for invalid in [
        "".to_string(),
        " ".repeat(2),
        "{\"name\":\"x\",\"tasks\":[]}".into(),
        "x".repeat(100001),
        "{\"name\":\"x\",\"tasks\":[{\"text\":\"raw\nnewline\"}]}".into(),
    ] {
        click(vc, "template-import");
        vc.write_to_clipboard(gpui::ClipboardItem::new_string(invalid));
        click(vc, "template-paste");
        click(vc, "accept");
        assert!(scene.read_with(vc, |s, _| s.form_error.is_some()));
        assert_eq!(
            disk(dir.path(), "templates.json").as_array().unwrap().len(),
            1
        );
        vc.simulate_keystrokes("escape");
        vc.run_until_parked();
    }
}
#[gpui::test]
fn template_native_file_prompts_cancel_export_actual_bytes_reimport(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let (scene, vc) = open(cx, dir.path());
    click(vc, "template-save");
    vc.simulate_keystrokes("a enter");
    vc.run_until_parked();
    click(vc, "template-choose");
    click(vc, "template-option-0");
    click(vc, "template-export");
    assert!(vc.did_prompt_for_new_path());
    vc.simulate_new_path_selection(|_| None);
    vc.run_until_parked();
    assert!(!dir.path().join("poe2-template-a.json").exists());
    click(vc, "template-export");
    let export = dir.path().join("poe2-template-a.json");
    vc.simulate_new_path_selection(|_| Some(export.clone()));
    vc.run_until_parked();
    let value = disk(dir.path(), "poe2-template-a.json");
    assert_eq!(value["version"], "2.0");
    assert_eq!(value["taskCount"], 3);
    click(vc, "template-import");
    click(vc, "template-file");
    assert!(vc.did_prompt_for_paths());
    vc.simulate_path_prompt_response(|options| {
        assert!(options.files && !options.directories && !options.multiple);
        None
    });
    vc.run_until_parked();
    assert!(scene.read_with(vc, |s, cx| s.editor.read(cx).text().is_empty()));
    click(vc, "template-file");
    vc.simulate_path_prompt_response(|_| Some(vec![export.clone()]));
    vc.run_until_parked();
    click(vc, "accept");
    click(vc, "accept");
    assert_eq!(
        disk(dir.path(), "templates.json")[0]["tasks"],
        value["tasks"]
    );
    assert_eq!(disk(dir.path(), "poe2-template-a.json"), value);
}
#[gpui::test]
fn template_community_all_six_preview_and_replacement_stay_open(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let (scene, vc) = open(cx, dir.path());
    click(vc, "template-community");
    for selector in [
        "community-import-0",
        "community-import-1",
        "community-import-2",
        "community-import-3",
        "community-import-4",
        "community-import-5",
    ] {
        // Scroll via keyboard focus, not offscreen click coordinates.
        scene.update_in(vc, |s, w, cx| w.focus(&s.controls[selector], cx));
        vc.simulate_keystrokes("space");
        vc.run_until_parked();
    }
    assert_eq!(
        disk(dir.path(), "templates.json").as_array().unwrap().len(),
        6
    );
    assert!(scene.read_with(vc, |s, _| s.dialog.is_some()));
    scene.update_in(vc, |s, w, cx| {
        w.focus(&s.controls["community-preview-0"], cx)
    });
    vc.simulate_keystrokes("space");
    vc.run_until_parked();
    vc.simulate_keystrokes("escape");
    vc.run_until_parked();
    assert!(scene.read_with(vc, |s, _| s.dialog.is_some()));
    scene.update_in(vc, |s, w, cx| {
        w.focus(&s.controls["community-import-0"], cx)
    });
    vc.simulate_keystrokes("space");
    vc.run_until_parked();
    vc.simulate_keystrokes("escape");
    vc.run_until_parked();
    assert_eq!(
        disk(dir.path(), "templates.json").as_array().unwrap().len(),
        6
    );
    scene.update_in(vc, |s, w, _| {
        assert!(s.controls["community-import-0"].is_focused(w) && s.dialog.is_some())
    });
    vc.simulate_keystrokes("space");
    vc.run_until_parked();
    click(vc, "accept");
    assert!(scene.read_with(vc, |s, _| s.dialog.is_some()));
}

#[gpui::test]
fn template_save_reload_load_nested_triggers_and_history(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let (_, vc) = open(cx, dir.path());
    click(vc, "template-save");
    vc.simulate_keystrokes("a l p h a enter");
    vc.run_until_parked();
    let templates = disk(dir.path(), "templates.json");
    assert_eq!(templates[0]["name"], "alpha");
    assert_eq!(
        templates[0]["tasks"][0]["children"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        templates[0]["tasks"][0]["children"][0]["triggerIndices"],
        json!([2])
    );
    let (scene, vc) = open(cx, dir.path());
    click(vc, "template-choose");
    click(vc, "template-option-0");
    click(vc, "template-load");
    vc.simulate_keystrokes("tab enter");
    assert!(scene.read_with(vc, |s, _| s.dialog.is_some()));
    click(vc, "accept");
    let data = disk(dir.path(), "current_tasks.json");
    assert_eq!(data["currentTemplate"], "alpha");
    assert_ne!(data["tasks"][0]["id"], 1);
    assert_eq!(
        data["tasks"][0]["children"][0]["triggers"][0],
        data["tasks"][1]["id"]
    );
    assert_eq!(data["tasks"][1]["activated"], false);
    vc.simulate_keystrokes("ctrl-shift-z");
    vc.run_until_parked();
    assert_eq!(
        disk(dir.path(), "current_tasks.json")["currentTemplate"],
        Value::Null
    );
    vc.simulate_keystrokes("ctrl-shift-y");
    vc.run_until_parked();
    assert_eq!(
        disk(dir.path(), "current_tasks.json")["currentTemplate"],
        "alpha"
    );
}
#[gpui::test]
fn template_import_retains_raw_newlines_after_typing_deletion_and_paste(cx: &mut TestAppContext) {
    use gpui::EntityInputHandler;
    for via_file in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        seed(dir.path());
        let (scene, vc) = open(cx, dir.path());
        click(vc, "template-import");
        let raw = "{\"name\":\"p\",\"tasks\":[{\"text\":\"🦀 \nline\"}]}";
        let source = dir.path().join("input.json");
        std::fs::write(&source, raw).unwrap();
        if via_file {
            click(vc, "template-file");
            vc.simulate_path_prompt_response(|_| Some(vec![source.clone()]));
            vc.run_until_parked();
        } else {
            vc.write_to_clipboard(gpui::ClipboardItem::new_string(raw.into()));
            vc.simulate_keystrokes("ctrl-v");
        }
        vc.simulate_keystrokes("space");
        vc.run_until_parked();
        click(vc, "accept");
        assert!(scene.read_with(vc, |s, _| s.form_error.is_some()));
        assert_eq!(disk(dir.path(), "templates.json"), json!([]));
        // Delete the ordinary space immediately before the newline, not the newline.
        let caret = raw.split('\n').next().unwrap().encode_utf16().count();
        scene.update_in(vc, |s, w, cx| {
            s.editor.update(cx, |input, cx| {
                input.set_selected_text_range(caret..caret, w, cx)
            });
            w.focus(&s.editor.focus_handle(cx), cx);
        });
        vc.simulate_keystrokes("backspace");
        vc.run_until_parked();
        click(vc, "accept");
        assert!(scene.read_with(vc, |s, _| s.form_error.is_some()));
        assert_eq!(disk(dir.path(), "templates.json"), json!([]));
        scene.update_in(vc, |s, w, cx| w.focus(&s.editor.focus_handle(cx), cx));
        vc.write_to_clipboard(gpui::ClipboardItem::new_string(" ".into()));
        vc.simulate_keystrokes("ctrl-v");
        click(vc, "accept");
        assert!(scene.read_with(vc, |s, _| s.form_error.is_some()));
        assert_eq!(disk(dir.path(), "templates.json"), json!([]));
        scene.update_in(vc, |s, w, cx| w.focus(&s.editor.focus_handle(cx), cx));
        vc.simulate_keystrokes("delete");
        vc.run_until_parked();
        click(vc, "accept");
        assert_eq!(
            disk(dir.path(), "templates.json")[0]["tasks"][0]["text"],
            "🦀 line"
        );
        assert_eq!(std::fs::read_to_string(source).unwrap(), raw);
    }
}

#[gpui::test]
fn template_multiline_buffer_is_visible_editable_and_roundtrips(cx: &mut TestAppContext) {
    use gpui::EntityInputHandler;
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let (scene, vc) = open(cx, dir.path());
    click(vc, "template-import");
    let raw = "{\n\"name\":\"multi\",\n\"tasks\":[{\"text\":\"🦀 \nline\"}]}";
    vc.write_to_clipboard(gpui::ClipboardItem::new_string(raw.into()));
    vc.simulate_keystrokes("ctrl-v");
    assert_eq!(
        scene.read_with(vc, |s, cx| s.editor.read(cx).text().to_owned()),
        raw
    );
    // Replacing a newline with a space must be a real edit, even adjacent to a space.
    let offset = raw[..raw.rfind('\n').unwrap()].encode_utf16().count();
    scene.update_in(vc, |s, w, cx| {
        s.editor.update(cx, |input, cx| {
            input.set_selected_text_range(offset..offset + 1, w, cx);
        })
    });
    vc.simulate_keystrokes("space");
    assert_eq!(
        scene.read_with(vc, |s, cx| s.editor.read(cx).text().to_owned()),
        raw.replacen(" \nline", "  line", 1)
    );
    vc.simulate_keystrokes("ctrl-a ctrl-c ctrl-x ctrl-v");
    click(vc, "accept");
    assert_eq!(
        disk(dir.path(), "templates.json")[0]["tasks"][0]["text"],
        "🦀  line"
    );
}

#[gpui::test]
fn template_multiline_parser_limit_uses_complete_utf16_buffer(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let (scene, vc) = open(cx, dir.path());
    click(vc, "template-import");
    let prefix = format!(
        "{{\n\"name\":\"limit\",\"tasks\":[{{\"text\":\"ok\"}}],\"ignored\":\"{}\"",
        "😀".repeat(49000)
    );
    let exact = format!(
        "{}{} }}",
        prefix,
        " ".repeat(100000 - prefix.encode_utf16().count() - 2)
    );
    assert_eq!(exact.encode_utf16().count(), 100000);
    let oversized = exact.replacen("{\n", "{\n ", 1);
    vc.write_to_clipboard(gpui::ClipboardItem::new_string(oversized.clone()));
    vc.simulate_keystrokes("ctrl-v");
    assert_eq!(
        scene.read_with(vc, |s, cx| s.editor.read(cx).text().to_owned()),
        oversized
    );
    click(vc, "accept");
    assert!(scene.read_with(vc, |s, _| {
        s.form_error
            .as_ref()
            .unwrap()
            .contains("Template data too large")
    }));
    assert_eq!(disk(dir.path(), "templates.json"), json!([]));
    scene.update_in(vc, |s, w, cx| w.focus(&s.editor.focus_handle(cx), cx));
    vc.simulate_keystrokes("ctrl-home down home delete");
    assert_eq!(
        scene.read_with(vc, |s, cx| s.editor.read(cx).text().to_owned()),
        exact
    );
    click(vc, "accept");
    assert_eq!(disk(dir.path(), "templates.json")[0]["name"], "limit");
}

#[gpui::test]
fn template_multiline_modal_trap_scroll_and_frozen_footer(cx: &mut TestAppContext) {
    use gpui::{EntityInputHandler, point};
    for scale in [1., 1.5, 2.] {
        let dir = tempfile::tempdir().unwrap();
        seed(dir.path());
        let (scene, vc) = open(cx, dir.path());
        vc.simulate_scale_factor_change(scale);
        click(vc, "template-import");
        let raw = format!(
            "{{\n{}\"name\":\"scroll\",\"tasks\":[{{\"text\":\"ok\"}}]}}",
            "\n".repeat(40)
        );
        vc.write_to_clipboard(gpui::ClipboardItem::new_string(raw.clone()));
        vc.simulate_keystrokes("ctrl-v");
        for dimensions in [size(px(500.), px(700.)), size(px(450.), px(600.))] {
            vc.simulate_resize(dimensions);
            vc.run_until_parked();
            let cancel = vc.debug_bounds("cancel").unwrap();
            let accept = vc.debug_bounds("accept").unwrap();
            let body = vc.debug_bounds("dialog-body").unwrap();
            for bounds in [cancel, accept] {
                assert!(bounds.top() >= px(0.) && bounds.bottom() <= dimensions.height);
            }
            let row = vc.debug_bounds("row-0").unwrap();
            // At this point the editor is the first item in the modal body.
            vc.simulate_event(gpui::ScrollEvent {
                position: body.origin + point(px(30.), px(30.)),
                delta: gpui::ScrollDelta::Lines(point(0., 100.)),
                ..Default::default()
            });
            vc.run_until_parked();
            assert_eq!(vc.debug_bounds("cancel").unwrap(), cancel);
            assert_eq!(vc.debug_bounds("accept").unwrap(), accept);
            assert_eq!(vc.debug_bounds("row-0").unwrap(), row);
            vc.simulate_click(body.origin + point(px(30.), px(30.)), Default::default());
            let selection = scene.update_in(vc, |s, w, cx| {
                s.editor.update(cx, |i, cx| {
                    i.selected_text_range(false, w, cx).unwrap().range
                })
            });
            assert!(
                selection.start < 4,
                "XY click after wheel should hit the first line, not the last"
            );
            for _ in 0..12 {
                vc.simulate_keystrokes("tab");
            }
            scene.update_in(vc, |s, w, _| {
                assert!(s.tab_order.iter().any(|(h, _)| h.is_focused(w)))
            });
            scene.update_in(vc, |s, w, cx| w.focus(&s.editor.focus_handle(cx), cx));
            vc.simulate_keystrokes("ctrl-end enter");
            assert!(scene.read_with(vc, |s, _| s.dialog.is_some()));
            assert_eq!(disk(dir.path(), "templates.json"), json!([]));
        }
        vc.simulate_keystrokes("escape");
        scene.update_in(vc, |s, w, _| {
            assert!(s.dialog.is_none());
            assert!(s.controls["template-import"].is_focused(w));
        });
    }
}

#[gpui::test]
fn template_silent_save_overwrite_delete_cancel_then_confirm(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let (scene, vc) = open(cx, dir.path());
    for _ in 0..2 {
        click(vc, "template-save");
        vc.simulate_keystrokes("a enter");
        vc.run_until_parked();
        assert!(scene.read_with(vc, |s, _| s.dialog.is_none()));
        assert_eq!(
            disk(dir.path(), "templates.json").as_array().unwrap().len(),
            1
        );
    }
    click(vc, "template-choose");
    click(vc, "template-option-0");
    click(vc, "template-delete");
    vc.simulate_keystrokes("tab enter escape");
    vc.run_until_parked();
    assert_eq!(
        disk(dir.path(), "templates.json").as_array().unwrap().len(),
        1
    );
    click(vc, "template-delete");
    click(vc, "accept");
    assert_eq!(disk(dir.path(), "templates.json"), json!([]));
}
