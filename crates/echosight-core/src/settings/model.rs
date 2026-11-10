use serde::{Deserialize, Serialize};

/// Current on-disk settings format. Read raw JSON through `normalize_settings`,
/// not directly through serde: old documents intentionally accept JS coercions.
pub const CURRENT_SETTINGS_VERSION: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HotkeyAction {
    ToggleVisibility,
    ToggleInteractive,
    CompleteNextTask,
    UndoLastAction,
    RedoLastAction,
}

impl HotkeyAction {
    pub const ALL: [Self; 5] = [
        Self::ToggleVisibility,
        Self::ToggleInteractive,
        Self::CompleteNextTask,
        Self::UndoLastAction,
        Self::RedoLastAction,
    ];

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|action| action.as_str() == value)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ToggleVisibility => "toggleVisibility",
            Self::ToggleInteractive => "toggleInteractive",
            Self::CompleteNextTask => "completeNextTask",
            Self::UndoLastAction => "undoLastAction",
            Self::RedoLastAction => "redoLastAction",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::ToggleVisibility => "Toggle Overlay Visibility",
            Self::ToggleInteractive => "Toggle Interactive Mode",
            Self::CompleteNextTask => "Complete Next Task",
            Self::UndoLastAction => "Undo Last Task Action",
            Self::RedoLastAction => "Forward Last Task Action",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeySettings {
    pub toggle_visibility: String,
    pub toggle_interactive: String,
    pub complete_next_task: String,
    pub undo_last_action: String,
    pub redo_last_action: String,
}

impl HotkeySettings {
    pub fn with_primary_modifier(primary: &str) -> Self {
        Self {
            toggle_visibility: format!("{primary}+Shift+T"),
            toggle_interactive: format!("{primary}+Shift+I"),
            complete_next_task: format!("{primary}+Shift+N"),
            undo_last_action: format!("{primary}+Shift+Z"),
            redo_last_action: format!("{primary}+Shift+Y"),
        }
    }

    pub fn get(&self, action: HotkeyAction) -> &str {
        match action {
            HotkeyAction::ToggleVisibility => &self.toggle_visibility,
            HotkeyAction::ToggleInteractive => &self.toggle_interactive,
            HotkeyAction::CompleteNextTask => &self.complete_next_task,
            HotkeyAction::UndoLastAction => &self.undo_last_action,
            HotkeyAction::RedoLastAction => &self.redo_last_action,
        }
    }

    pub fn set(&mut self, action: HotkeyAction, hotkey: String) {
        let field = match action {
            HotkeyAction::ToggleVisibility => &mut self.toggle_visibility,
            HotkeyAction::ToggleInteractive => &mut self.toggle_interactive,
            HotkeyAction::CompleteNextTask => &mut self.complete_next_task,
            HotkeyAction::UndoLastAction => &mut self.undo_last_action,
            HotkeyAction::RedoLastAction => &mut self.redo_last_action,
        };
        *field = hotkey;
    }
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self::with_primary_modifier("CommandOrControl")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundSettings {
    pub enabled: bool,
    pub volume: u8,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            volume: 60,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub settings_version: u8,
    pub transparency: u8,
    pub theme: String,
    pub hotkeys: HotkeySettings,
    pub sounds: SoundSettings,
}

impl Settings {
    pub fn with_hotkeys(hotkeys: HotkeySettings) -> Self {
        Self {
            settings_version: CURRENT_SETTINGS_VERSION,
            transparency: 70,
            theme: "echosight".into(),
            hotkeys,
            sounds: SoundSettings::default(),
        }
    }
}

/// Bootstrap defaults use portable accelerators, unlike renderer labels.
impl Default for Settings {
    fn default() -> Self {
        Self::with_hotkeys(HotkeySettings::default())
    }
}
