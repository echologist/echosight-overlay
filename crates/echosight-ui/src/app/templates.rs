//! Template workflow adapter. Algorithms and wire formats belong to core.
use super::*;
use echosight_core::template::{
    community::*,
    model::{TaskTemplate, TemplateData},
    ops::*,
    persistence::*,
    transfer::*,
};
use gpui::{EntityInputHandler, SharedString, div};

#[derive(Clone)]
pub(super) enum TemplateCommand {
    Choose,
    Select(usize),
    Save,
    Load,
    Delete,
    Import,
    File,
    Paste,
    Export,
    Community,
    CommunityImport(usize),
    Preview(usize),
}
#[derive(Clone)]
pub(super) enum TemplateDialog {
    Choose,
    Save,
    Load(TaskTemplate),
    Delete(TaskTemplate),
    Import,
    Community,
    Replace(Incoming),
    Preview(String),
}
#[derive(Clone)]
pub(super) enum Incoming {
    Import(TemplateData),
    Community(usize),
}
impl Incoming {
    fn name(&self) -> &str {
        match self {
            Self::Import(data) => &data.name,
            Self::Community(i) => COMMUNITY_TEMPLATES[*i].name,
        }
    }
}
impl TemplateDialog {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Choose => "Select Template...",
            Self::Save => "Save Template",
            Self::Load(_) => "Load Template",
            Self::Delete(_) => "Delete Template",
            Self::Import => "Import Template",
            Self::Community => "Community Templates",
            Self::Replace(_) => "Confirm",
            Self::Preview(_) => "Preview",
        }
    }
    pub fn danger(&self) -> bool {
        matches!(self, Self::Load(_) | Self::Delete(_) | Self::Replace(_))
    }
}
#[derive(Default)]
pub(super) struct Templates {
    pub collection: Option<Vec<TaskTemplate>>,
    pub selected: Option<usize>,
    pub error: Option<String>,
    pub saving: bool,
    pub transferring: bool,
    pub ports: Option<std::rc::Rc<dyn super::ports::TemplatePorts>>,
    directory: PathBuf,
    clock: Option<TaskClock>,
    pending: VecDeque<TemplateWrite>,
    generation: u64,
}
struct TemplateWrite {
    next: Vec<TaskTemplate>,
    message: String,
    generation: u64,
    close: bool,
}
impl TaskApp {
    fn template_editor(
        import: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<TextInput> {
        let editor = cx.new(|cx| {
            let input = TextInput::new(cx)
                .placeholder(if import {
                    "Paste template JSON or share code here..."
                } else {
                    "Template name..."
                })
                .style(view::input_style());
            if import {
                input.multiline(8)
            } else {
                input.max_length(50)
            }
        });
        cx.subscribe_in(
            &editor,
            window,
            move |this, _, event, window, cx| match event {
                TextInputEvent::Submit(_) => this.activate(Command::Accept, window, cx),
                TextInputEvent::Cancel => this.close_dialog(window, cx),
                TextInputEvent::Changed if import => {
                    this.form_error = None;
                    cx.notify();
                }
                _ => {}
            },
        )
        .detach();
        editor
    }
    fn push_template_dialog(
        &mut self,
        dialog: TemplateDialog,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(parent) = self.dialog.take() {
            self.modal_stack.push(triggers::ModalFrame {
                dialog: parent,
                editor: self.editor.clone(),
                high_priority: self.high_priority,
                restore_focus: self.restore_focus.take(),
            });
        }
        self.open_template_dialog(dialog, window, cx);
    }
    fn receive_template(
        &mut self,
        incoming: Incoming,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if replacement_prompt(self.templates.collection.as_ref().unwrap(), incoming.name())
            .is_some()
        {
            self.push_template_dialog(TemplateDialog::Replace(incoming), window, cx);
        } else {
            self.persist_incoming(incoming, window, cx);
        }
    }
    fn persist_incoming(
        &mut self,
        incoming: Incoming,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let clock = self
            .templates
            .clock
            .as_ref()
            .expect("loaded templates have clock");
        let (template, message) = match incoming {
            Incoming::Import(data) => {
                let t = create_imported_template(data, clock);
                let message = format!(
                    "Template \"{}\" imported successfully! ({} tasks)",
                    t.name,
                    count_template_tasks(&t.tasks)
                );
                (t, message)
            }
            Incoming::Community(index) => {
                let catalog = &COMMUNITY_TEMPLATES[index];
                let t = create_community_template(
                    catalog,
                    create_community_template_tasks(catalog),
                    clock,
                );
                let message = format!("Community template \"{}\" added to your templates!", t.name);
                (t, message)
            }
        };
        let next = upsert_template_by_name(self.templates.collection.as_ref().unwrap(), template);
        self.queue_template_write(next, message, window, cx);
    }
    fn set_import_text(&mut self, raw: String, window: &mut Window, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |input, cx| input.set_text(raw, window, cx));
        self.form_error = None;
        window.focus(&self.editor.focus_handle(cx), cx);
        cx.notify();
    }
    pub(super) fn paste_template(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !matches!(self.dialog, Some(Dialog::Template(TemplateDialog::Import)))
            || !self.templates_ready()
        {
            return;
        }
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.editor.update(cx, |input, cx| {
                input.replace_text_in_range(None, &text, window, cx)
            });
            window.focus(&self.editor.focus_handle(cx), cx);
        }
    }
    fn import_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let prompt = self.templates.ports.as_ref().unwrap().prompt_open_file(cx);
        let generation = self.templates.generation;
        self.templates.transferring = true;
        cx.spawn_in(window, async move |this, cx| {
            let result = match prompt.await {
                Ok(Some(path)) => {
                    cx.background_executor()
                        .spawn(async move {
                            std::fs::read_to_string(path)
                                .map(Some)
                                .map_err(|e| e.to_string())
                        })
                        .await
                }
                other => other.map(|_| None),
            };
            let _ = this.update_in(cx, |this, window, cx| {
                this.templates.transferring = false;
                if this.templates.generation == generation
                    && matches!(this.dialog, Some(Dialog::Template(TemplateDialog::Import)))
                {
                    match result {
                        Ok(Some(text)) => this.set_import_text(text, window, cx),
                        Ok(None) => {}
                        Err(e) => {
                            this.form_error =
                                Some(format!("Error reading file. Please try again. {e}"))
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn export_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(template) = self.selected_template() else {
            self.status = "Please select a template to export!".into();
            return;
        };
        let export = match export_template(&template, self.templates.clock.as_ref().unwrap()) {
            Ok(export) => export,
            Err(e) => {
                self.templates.error =
                    Some(format!("Error exporting template. Please try again. {e}"));
                return;
            }
        };
        let prompt = self.templates.ports.as_ref().unwrap().prompt_save_file(
            &self.templates.directory,
            &export.filename,
            cx,
        );
        self.dragging = None;
        self.templates.transferring = true;
        cx.spawn_in(window, async move |this, cx| {
            let result = match prompt.await {
                Ok(Some(path)) => {
                    cx.background_executor()
                        .spawn(async move {
                            std::fs::write(path, export.bytes)
                                .map(|_| true)
                                .map_err(|e| e.to_string())
                        })
                        .await
                }
                Ok(None) => Ok(false),
                Err(e) => Err(e),
            };
            let _ = this.update_in(cx, |this, _, cx| {
                this.templates.transferring = false;
                match result {
                    Ok(true) => {
                        this.status =
                            format!("Template \"{}\" exported successfully!", template.name)
                    }
                    Ok(false) => {}
                    Err(e) => {
                        this.templates.error =
                            Some(format!("Error exporting template. Please try again. {e}"))
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
    pub(super) fn templates_closed(&mut self) {
        self.templates.generation = self.templates.generation.wrapping_add(1);
    }
    pub(super) fn danger_dialog(&self) -> bool {
        matches!(self.dialog, Some(Dialog::Delete(_) | Dialog::Quit))
            || matches!(&self.dialog, Some(Dialog::Template(dialog)) if dialog.danger())
    }
    pub(super) fn load_templates(
        &mut self,
        storage: Storage,
        paths: DataPaths,
        mut clock: TaskClock,
        cx: &mut Context<Self>,
    ) {
        self.templates.directory = paths.data_dir().to_path_buf();
        self.templates.clock = Some(clock.clone());
        self.templates.ports = Some(std::rc::Rc::new(super::ports::NativeTemplatePorts));
        let task = cx.background_executor().spawn(async move {
            let raw = persistence::load_templates(&storage, &paths);
            let result = resolve_template_load(raw.as_ref().map_err(String::as_str), &mut clock)
                .map_err(|e| e.to_string())?;
            if let Some(diagnostic) = result.diagnostic {
                return Err(format!("{} {}", diagnostic.prefix, diagnostic.message));
            }
            Ok(result.result)
        });
        cx.spawn(async move |this, cx| {
            let result: Result<Vec<TaskTemplate>, String> = task.await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(templates) => this.templates.collection = Some(templates),
                    Err(e) => this.templates.error = Some(e),
                }
                cx.notify();
            });
        })
        .detach();
    }
    pub(super) fn templates_ready(&self) -> bool {
        self.templates.collection.is_some()
            && !self.templates.saving
            && !self.templates.transferring
    }
    fn selected_template(&self) -> Option<TaskTemplate> {
        let collection = self.templates.collection.as_ref()?;
        let id = collection.get(self.templates.selected?)?.id;
        // Renderer selects string IDs, ignores falsy zero, and resolves the first
        // matching ID even when multiple options share it. Do not fix that quirk.
        if id.0 == 0.0 {
            return None;
        }
        find_template_by_id(collection, id).cloned()
    }
    fn open_template_dialog(
        &mut self,
        dialog: TemplateDialog,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.templates.generation = self.templates.generation.wrapping_add(1);
        self.restore_focus = window.focused(cx);
        self.dragging = None;
        self.form_error = None;
        let editing = matches!(dialog, TemplateDialog::Save | TemplateDialog::Import);
        let import = matches!(dialog, TemplateDialog::Import);
        self.dialog = Some(Dialog::Template(dialog));
        let focus = if editing {
            self.editor = Self::template_editor(import, window, cx);
            self.editor.focus_handle(cx)
        } else {
            self.controls
                .entry("cancel".into())
                .or_insert_with(|| cx.focus_handle())
                .clone()
        };
        window.focus(&focus, cx);
        cx.notify();
    }
    pub(super) fn template_command(
        &mut self,
        command: TemplateCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.templates_ready() {
            return;
        }
        let nested = matches!(
            (&self.dialog, &command),
            (
                Some(Dialog::Template(TemplateDialog::Choose)),
                TemplateCommand::Select(_)
            ) | (
                Some(Dialog::Template(TemplateDialog::Import)),
                TemplateCommand::File | TemplateCommand::Paste
            ) | (
                Some(Dialog::Template(TemplateDialog::Community)),
                TemplateCommand::CommunityImport(_) | TemplateCommand::Preview(_)
            )
        );
        if self.dialog.is_some() && !nested {
            return;
        }
        match command {
            TemplateCommand::Import => {
                self.open_template_dialog(TemplateDialog::Import, window, cx)
            }
            TemplateCommand::Community => {
                self.open_template_dialog(TemplateDialog::Community, window, cx)
            }
            TemplateCommand::File => self.import_file(window, cx),
            TemplateCommand::Paste => self.paste_template(window, cx),
            TemplateCommand::Export => self.export_selected(window, cx),
            TemplateCommand::CommunityImport(index) => {
                if index < COMMUNITY_TEMPLATES.len() {
                    self.receive_template(Incoming::Community(index), window, cx);
                }
            }
            TemplateCommand::Preview(index) => {
                if let Some(t) = COMMUNITY_TEMPLATES.get(index) {
                    self.push_template_dialog(
                        TemplateDialog::Preview(create_community_template_preview(t)),
                        window,
                        cx,
                    );
                }
            }
            TemplateCommand::Choose => {
                self.open_template_dialog(TemplateDialog::Choose, window, cx)
            }
            TemplateCommand::Select(index) => {
                if !matches!(self.dialog, Some(Dialog::Template(TemplateDialog::Choose))) {
                    return;
                }
                if index < self.templates.collection.as_ref().unwrap().len() {
                    self.templates.selected = Some(index);
                }
                self.close_dialog(window, cx);
            }
            TemplateCommand::Save => {
                if !self.writable() {
                    return;
                }
                if self.state().unwrap().tasks().is_empty() {
                    self.status = "No tasks to save as template!".into();
                } else {
                    self.open_template_dialog(TemplateDialog::Save, window, cx);
                }
            }
            TemplateCommand::Load => {
                if !self.writable() {
                    return;
                }
                if let Some(template) = self.selected_template() {
                    if echosight_core::task::completion::has_incomplete_tasks(
                        self.state().unwrap().tasks(),
                    ) {
                        self.open_template_dialog(TemplateDialog::Load(template), window, cx);
                    } else {
                        self.load_selected_template(template, cx);
                    }
                }
            }
            TemplateCommand::Delete => {
                if let Some(template) = self.selected_template() {
                    self.open_template_dialog(TemplateDialog::Delete(template), window, cx);
                } else {
                    self.status = "Please select a template to delete!".into();
                }
            }
        }
        cx.notify();
    }
    pub(super) fn accept_template(
        &mut self,
        dialog: TemplateDialog,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.templates_ready() {
            return;
        }
        let collection = self.templates.collection.as_ref().unwrap();
        match dialog {
            TemplateDialog::Choose | TemplateDialog::Community => {}
            TemplateDialog::Preview(_) => self.close_dialog(window, cx),
            TemplateDialog::Replace(incoming) => {
                self.close_dialog(window, cx);
                self.persist_incoming(incoming, window, cx);
            }
            TemplateDialog::Import => {
                let text = self.editor.read(cx).text();
                if echosight_core::jscompat::js_trim(text).is_empty() {
                    self.form_error =
                        Some("Please paste template JSON, share code, or select a file!".into());
                } else {
                    match parse_template_import(text) {
                        Ok(data) => self.receive_template(Incoming::Import(data), window, cx),
                        Err(e) => self.form_error = Some(format!("Invalid template format: {e}")),
                    }
                }
                cx.notify();
            }
            TemplateDialog::Save => {
                if !self.writable() {
                    return;
                }
                let name =
                    echosight_core::jscompat::js_trim(self.editor.read(cx).text()).to_owned();
                if name.is_empty() {
                    self.form_error = Some("Please enter a template name!".into());
                    cx.notify();
                    return;
                }
                let state = self.state().unwrap();
                if state.tasks().is_empty() {
                    self.form_error = Some("No tasks to save!".into());
                    cx.notify();
                    return;
                }
                let template =
                    create_template_from_tasks(name.clone(), state.tasks(), &state.environment);
                let next = upsert_template_by_name(collection, template);
                self.queue_template_write(
                    next,
                    format!("Template \"{name}\" saved successfully!"),
                    window,
                    cx,
                );
            }
            TemplateDialog::Load(template) => {
                if !self.writable() {
                    return;
                }
                self.close_dialog(window, cx);
                self.load_selected_template(template, cx);
            }
            TemplateDialog::Delete(template) => {
                let next = remove_template_by_id(collection, template.id);
                self.queue_template_write(
                    next,
                    format!("Template \"{}\" deleted successfully!", template.name),
                    window,
                    cx,
                );
            }
        }
    }
    fn load_selected_template(&mut self, template: TaskTemplate, cx: &mut Context<Self>) {
        // Source clears captured closures before replacement. Newly instantiated
        // background tasks are dormant: templateController does NOT restart timers.
        let mut clock = self.state().unwrap().environment.clone();
        let tasks = instantiate_template_tasks(&template, &mut clock);
        self.dispatch_background(BackgroundCommand::CancelAll, cx);
        self.dispatch_background(BackgroundCommand::CaptureUndo("load template".into()), cx);
        self.dispatch_background(
            BackgroundCommand::Replace {
                tasks,
                current_template: Some(template.name),
            },
            cx,
        );
        self.dispatch_background(BackgroundCommand::Save, cx);
    }
    fn queue_template_write(
        &mut self,
        next: Vec<TaskTemplate>,
        message: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.templates.pending.push_back(TemplateWrite {
            next,
            message,
            generation: self.templates.generation,
            close: !matches!(
                self.dialog,
                Some(Dialog::Template(TemplateDialog::Community))
            ),
        });
        self.pump_templates(window, cx);
    }
    fn pump_templates(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.templates.saving {
            return;
        }
        let Some(write) = self.templates.pending.pop_front() else {
            return;
        };
        let storage = self.storage.clone().expect("template load has storage");
        let data = serde_json::to_value(&write.next).expect("normalized template collection");
        self.templates.saving = true;
        let task = cx.background_executor().spawn(async move {
            storage
                .save(FileKind::Templates, &data)
                .map_err(|e| e.to_string())
        });
        cx.spawn_in(window, async move |this, cx| {
            let saved = task.await;
            let outcome = resolve_template_save(
                saved
                    .as_ref()
                    .map(|_| TemplateSaveResponse {
                        success: true,
                        error: None,
                    })
                    .map_err(String::as_str),
            );
            let _ = this.update_in(cx, |this, window, cx| {
                this.templates.saving = false;
                if outcome.result {
                    this.templates.collection = Some(write.next);
                    this.templates.selected = None; // renderTemplateSelect resets selection.
                    this.templates.error = None;
                    this.status = write.message;
                    if write.close
                        && this.templates.generation == write.generation
                        && matches!(this.dialog, Some(Dialog::Template(_)))
                    {
                        this.close_dialog(window, cx);
                    }
                } else {
                    let diagnostic = outcome.diagnostic.unwrap();
                    this.templates.error =
                        Some(format!("{} {}", diagnostic.prefix, diagnostic.message));
                }
                this.pump_templates(window, cx);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    pub(super) fn template_bar(&mut self, window: &Window, cx: &mut Context<Self>) -> gpui::Div {
        let ready = self.templates_ready() && self.dialog.is_none();
        let label = self
            .selected_template()
            .map(|t| format!("{} ({} tasks)", t.name, count_template_tasks(&t.tasks)))
            .unwrap_or_else(|| "Select Template...".into());
        let mut bar = div()
            .flex()
            .flex_col()
            .gap_1()
            .flex_shrink_0()
            .child(self.button(
                "template-choose".into(),
                label,
                Command::Template(TemplateCommand::Choose),
                ready,
                window,
                cx,
            ));
        let mut actions = div().flex().flex_wrap().gap_1();
        for (key, label, command) in [
            ("template-load", "Load", TemplateCommand::Load),
            ("template-save", "Save", TemplateCommand::Save),
            ("template-delete", "Delete", TemplateCommand::Delete),
            ("template-import", "Import", TemplateCommand::Import),
            ("template-export", "Export", TemplateCommand::Export),
            (
                "template-community",
                "Community",
                TemplateCommand::Community,
            ),
        ] {
            actions = actions.child(self.button(
                key.into(),
                label,
                Command::Template(command),
                ready,
                window,
                cx,
            ));
        }
        bar = bar.child(actions);
        if let Some(e) = &self.templates.error {
            bar = bar.child(div().text_xs().child(e.clone()));
        }
        if self.templates.saving {
            bar = bar.child("Saving templates…");
        }
        bar
    }
    pub(super) fn template_fields(
        &mut self,
        dialog: &TemplateDialog,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let mut fields = div().flex().flex_col().gap_2();
        match dialog {
            TemplateDialog::Import => {
                self.tab_order.push((self.editor.focus_handle(cx), None));
                fields = fields
                    .child(self.editor.clone())
                    .child(self.button(
                        "template-paste".into(),
                        "Paste",
                        Command::Template(TemplateCommand::Paste),
                        self.templates_ready(),
                        window,
                        cx,
                    ))
                    .child(self.button(
                        "template-file".into(),
                        "Choose File",
                        Command::Template(TemplateCommand::File),
                        self.templates_ready(),
                        window,
                        cx,
                    ));
            }
            TemplateDialog::Community => {
                for (i, t) in COMMUNITY_TEMPLATES.iter().enumerate() {
                    fields = fields.child(
                        div()
                            .child(t.name)
                            .child(div().text_xs().child(t.description))
                            .child(self.button(
                                format!("community-preview-{i}"),
                                "Preview",
                                Command::Template(TemplateCommand::Preview(i)),
                                self.templates_ready(),
                                window,
                                cx,
                            ))
                            .child(self.button(
                                format!("community-import-{i}"),
                                "Import",
                                Command::Template(TemplateCommand::CommunityImport(i)),
                                self.templates_ready(),
                                window,
                                cx,
                            )),
                    );
                }
            }
            TemplateDialog::Replace(incoming) => {
                fields = fields.child(format!(
                    "Template \"{}\" already exists. Replace it?",
                    incoming.name()
                ));
            }
            TemplateDialog::Preview(message) => {
                fields = fields.child(message.clone());
            }
            TemplateDialog::Save => {
                self.tab_order.push((self.editor.focus_handle(cx), None));
                fields = fields.child(self.editor.clone());
            }
            TemplateDialog::Choose => {
                for (i, template) in self
                    .templates
                    .collection
                    .clone()
                    .unwrap_or_default()
                    .iter()
                    .enumerate()
                {
                    fields = fields.child(self.button(
                        format!("template-option-{i}"),
                        format!(
                            "{} ({} tasks)",
                            template.name,
                            count_template_tasks(&template.tasks)
                        ),
                        Command::Template(TemplateCommand::Select(i)),
                        self.templates_ready(),
                        window,
                        cx,
                    ));
                }
            }
            TemplateDialog::Load(_) => {
                fields = fields.child("This will replace your current tasks. Continue?");
            }
            TemplateDialog::Delete(t) => {
                fields = fields.child(div().id("dialog-message").debug_selector(|| "dialog-message".into()).child(SharedString::from(format!("Are you sure you want to delete the template \"{}\"? This cannot be undone.", t.name))));
            }
        }
        fields
    }
}
