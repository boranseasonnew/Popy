use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Setup,
    Add,
    Preview,
    Start,
    Stop,
    ChooseOutput,
    OpenVideo,
    OpenFolder,
    Retry,
    Remove,
    MoveUp,
    MoveDown,
    Previous,
    Next,
    Color,
    General,
    Anime,
    Settings,
    Help,
    Theme,
    Motion,
    Companion,
    Before,
    After,
    Fit,
    ZoomIn,
    ZoomOut,
    Updates,
    Fullscreen,
}
impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Self::Setup => "setup_tools",
            Self::Add => "add",
            Self::Preview => "preview",
            Self::Start => "enhance",
            Self::Stop => "stop",
            Self::ChooseOutput => "choose_folder",
            Self::OpenVideo => "open_video",
            Self::OpenFolder => "open_folder",
            Self::Retry => "retry",
            Self::Remove => "remove",
            Self::MoveUp => "move_up",
            Self::MoveDown => "move_down",
            Self::Previous => "previous",
            Self::Next => "next",
            Self::Color => "color",
            Self::General => "model_general",
            Self::Anime => "model_anime",
            Self::Settings => "settings",
            Self::Help => "shortcuts",
            Self::Theme => "theme",
            Self::Motion => "reduced_motion",
            Self::Companion => "companion",
            Self::Before => "source",
            Self::After => "result",
            Self::Fit => "fit",
            Self::ZoomIn => "zoom_in",
            Self::ZoomOut => "zoom_out",
            Self::Updates => "check_updates",
            Self::Fullscreen => "fullscreen",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Binding {
    pub action: Action,
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}
impl Binding {
    pub fn shortcut(&self) -> Option<KeyboardShortcut> {
        Some(KeyboardShortcut::new(
            Modifiers {
                ctrl: self.ctrl,
                command: self.ctrl,
                shift: self.shift,
                alt: self.alt,
                ..Modifiers::NONE
            },
            Key::from_name(&self.key)?,
        ))
    }
    pub fn display(&self) -> String {
        format!(
            "{}{}{}{}",
            if self.ctrl { "Ctrl+" } else { "" },
            if self.alt { "Alt+" } else { "" },
            if self.shift { "Shift+" } else { "" },
            self.key
        )
    }
    pub fn from_event(action: Action, key: Key, modifiers: Modifiers) -> Self {
        Self {
            action,
            key: key.name().into(),
            ctrl: modifiers.ctrl || modifiers.command,
            shift: modifiers.shift,
            alt: modifiers.alt,
        }
    }
    fn same_keys(&self, other: &Self) -> bool {
        self.key == other.key
            && self.ctrl == other.ctrl
            && self.shift == other.shift
            && self.alt == other.alt
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Shortcuts {
    pub bindings: Vec<Binding>,
}
impl Default for Shortcuts {
    fn default() -> Self {
        use Action::*;
        let definitions = [
            (Setup, "F8", false, false, false),
            (Add, "O", true, false, false),
            (Preview, "F5", false, false, false),
            (Start, "Enter", true, false, false),
            (Stop, "X", true, true, false),
            (ChooseOutput, "S", true, true, false),
            (OpenVideo, "Enter", true, true, false),
            (OpenFolder, "O", true, true, false),
            (Retry, "R", true, false, false),
            (Remove, "Delete", false, false, false),
            (MoveUp, "ArrowUp", false, false, true),
            (MoveDown, "ArrowDown", false, false, true),
            (Previous, "ArrowLeft", false, false, true),
            (Next, "ArrowRight", false, false, true),
            (Color, "C", false, false, true),
            (General, "1", false, false, true),
            (Anime, "2", false, false, true),
            (Settings, "Comma", true, false, false),
            (Help, "F1", false, false, false),
            (Theme, "F6", false, false, false),
            (Motion, "M", true, true, false),
            (Companion, "B", true, false, false),
            (Before, "B", false, false, false),
            (After, "A", false, false, false),
            (Fit, "0", true, false, false),
            (ZoomIn, "Plus", true, false, false),
            (ZoomOut, "Minus", true, false, false),
            (Updates, "U", true, false, false),
            (Fullscreen, "F11", false, false, false),
        ];
        Self {
            bindings: definitions
                .into_iter()
                .map(|(action, key, ctrl, shift, alt)| Binding {
                    action,
                    key: key.into(),
                    ctrl,
                    shift,
                    alt,
                })
                .collect(),
        }
    }
}
impl Shortcuts {
    pub fn fill_missing_defaults(&mut self) {
        for mut binding in Self::default().bindings {
            if !self.bindings.iter().any(|b| b.action == binding.action) {
                if self.bindings.iter().any(|b| b.same_keys(&binding)) {
                    binding.key.clear();
                }
                self.bindings.push(binding);
            }
        }
    }
    pub fn label(&self, action: Action) -> String {
        self.bindings
            .iter()
            .find(|b| b.action == action)
            .map(Binding::display)
            .unwrap_or_default()
    }
    pub fn rebind(&mut self, binding: Binding) -> Result<(), Action> {
        if let Some(conflict) = self
            .bindings
            .iter()
            .find(|b| b.action != binding.action && b.same_keys(&binding))
        {
            return Err(conflict.action);
        }
        if let Some(existing) = self
            .bindings
            .iter_mut()
            .find(|b| b.action == binding.action)
        {
            *existing = binding;
        } else {
            self.bindings.push(binding);
        }
        Ok(())
    }
    pub fn consume(&self, ctx: &egui::Context) -> Vec<Action> {
        let editing = ctx.text_edit_focused();
        let mut fired = Vec::new();
        // More-specific modifier combinations first: Ctrl+Shift+Enter must not
        // be consumed by Ctrl+Enter (egui allows extra shift modifiers).
        let mut sorted: Vec<_> = self.bindings.iter().collect();
        sorted.sort_by_key(|b| {
            std::cmp::Reverse(u8::from(b.ctrl) + u8::from(b.shift) + u8::from(b.alt))
        });
        for binding in sorted {
            if editing && !binding.ctrl && !binding.alt {
                continue;
            }
            if let Some(shortcut) = binding.shortcut()
                && ctx.input_mut(|i| i.consume_shortcut(&shortcut))
            {
                fired.push(binding.action);
            }
        }
        fired
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_actions_do_not_steal_an_existing_custom_shortcut() {
        let mut shortcuts = Shortcuts::default();
        shortcuts.bindings.retain(|b| b.action != Action::Setup);
        shortcuts
            .rebind(Binding {
                action: Action::Add,
                key: "F8".into(),
                ctrl: false,
                shift: false,
                alt: false,
            })
            .unwrap();
        shortcuts.fill_missing_defaults();
        assert_eq!(shortcuts.label(Action::Add), "F8");
        assert!(
            shortcuts
                .bindings
                .iter()
                .find(|b| b.action == Action::Setup)
                .unwrap()
                .shortcut()
                .is_none()
        );
    }
    #[test]
    fn defaults_have_valid_keys_and_no_collisions() {
        let shortcuts = Shortcuts::default();
        for (i, binding) in shortcuts.bindings.iter().enumerate() {
            assert!(binding.shortcut().is_some(), "{}", binding.key);
            assert!(
                !shortcuts
                    .bindings
                    .iter()
                    .skip(i + 1)
                    .any(|other| binding.same_keys(other))
            );
        }
    }
    #[test]
    fn conflicting_remaps_are_rejected_without_changing_the_original() {
        let mut shortcuts = Shortcuts::default();
        let original = shortcuts.label(Action::Start);
        let mut binding = shortcuts
            .bindings
            .iter()
            .find(|b| b.action == Action::Add)
            .unwrap()
            .clone();
        binding.action = Action::Start;
        assert_eq!(shortcuts.rebind(binding), Err(Action::Add));
        assert_eq!(shortcuts.label(Action::Start), original);
    }
}
