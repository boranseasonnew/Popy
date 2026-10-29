use super::*;
use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use popy::{
    i18n,
    shortcuts::{Action, Binding},
    updater::{self, StagedUpdate, UpdateManifest},
};
use std::sync::{atomic::AtomicBool, mpsc};

#[derive(Clone, Copy)]
struct Palette {
    bg: Color32,
    surface: Color32,
    soft: Color32,
    text: Color32,
    muted: Color32,
    line: Color32,
    accent: Color32,
    tint: Color32,
    canvas: Color32,
    good: Color32,
    bad: Color32,
}
fn color(light: [u8; 3], dark: [u8; 3], t: f32) -> Color32 {
    let channel =
        |i: usize| (light[i] as f32 + (dark[i] as f32 - light[i] as f32) * t).round() as u8;
    Color32::from_rgb(channel(0), channel(1), channel(2))
}
impl Palette {
    fn new(t: f32) -> Self {
        Self {
            bg: color([247, 244, 239], [26, 25, 30], t),
            surface: color([255, 254, 252], [37, 35, 40], t),
            soft: color([239, 233, 225], [49, 43, 44], t),
            text: color([58, 45, 40], [243, 235, 225], t),
            muted: color([144, 128, 116], [166, 153, 143], t),
            line: color([230, 222, 213], [62, 53, 51], t),
            accent: color([222, 111, 64], [247, 151, 92], t),
            tint: color([255, 231, 210], [74, 46, 35], t),
            canvas: color([235, 230, 222], [20, 20, 25], t),
            good: color([76, 133, 98], [133, 194, 154], t),
            bad: color([186, 84, 79], [233, 144, 135], t),
        }
    }
}
enum UpdateEvent {
    Checked(Result<Option<UpdateManifest>, String>),
    Downloaded(Result<StagedUpdate, String>),
    Progress(f32),
}
#[derive(Default)]
struct UpdateUi {
    status: &'static str,
    manifest: Option<UpdateManifest>,
    staged: Option<StagedUpdate>,
    receiver: Option<mpsc::Receiver<UpdateEvent>>,
    cancel: Arc<AtomicBool>,
    fraction: f32,
    error: String,
    auto_install: bool,
}
pub(super) struct Studio {
    pub(super) settings_open: bool,
    tab: u8,
    capture: Option<Action>,
    conflict: Option<Action>,
    zoom: f32,
    fullscreen: bool,
    fox: Option<TextureHandle>,
    bear: Option<TextureHandle>,
    update: UpdateUi,
    started: Instant,
    completed: Option<Instant>,
    notice: Option<(&'static str, Instant)>,
    runtime_check: Instant,
    keyboard_step: usize,
    pub(super) update_test: Option<PathBuf>,
    setup: Option<ManagedChild>,
    setup_state: &'static str,
}
impl Studio {
    pub(super) fn new(ctx: &egui::Context) -> Self {
        Self {
            settings_open: false,
            tab: 0,
            capture: None,
            conflict: None,
            zoom: 1.0,
            fullscreen: false,
            fox: texture(
                ctx,
                include_bytes!("../branding/companions/fox-v1.png"),
                "fox-companion",
            ),
            bear: texture(
                ctx,
                include_bytes!("../branding/companions/bear-v1.png"),
                "bear-companion",
            ),
            update: Default::default(),
            started: Instant::now(),
            completed: None,
            notice: None,
            runtime_check: Instant::now(),
            keyboard_step: 0,
            update_test: None,
            setup: None,
            setup_state: "",
        }
    }
}
fn configure(ctx: &egui::Context, prefs: &Preferences) -> Palette {
    let target = if prefs.dark { 1.0 } else { 0.0 };
    let mix = ctx.animate_value_with_time(
        egui::Id::unique("theme-transition"),
        target,
        if prefs.reduced_motion { 0.0 } else { 0.24 },
    );
    let p = Palette::new(mix);
    let theme = if prefs.dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    let mut style = (*ctx.style_of(theme)).clone();
    style.visuals = if prefs.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    style.visuals.override_text_color = Some(p.text);
    style.visuals.panel_fill = p.bg;
    style.visuals.window_fill = p.surface;
    style.visuals.window_corner_radius = 18.into();
    style.visuals.window_stroke = Stroke::new(1.0, p.line);
    style.visuals.selection.bg_fill = p.tint;
    style.visuals.selection.stroke = Stroke::new(1.4, p.accent);
    style.visuals.widgets.inactive.bg_fill = p.soft;
    style.visuals.widgets.inactive.weak_bg_fill = p.soft;
    style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    style.visuals.widgets.hovered.bg_fill = p.tint;
    style.visuals.widgets.hovered.weak_bg_fill = p.tint;
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent.gamma_multiply(0.3));
    style.visuals.widgets.active.bg_fill = p.tint;
    style.visuals.widgets.active.weak_bg_fill = p.tint;
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.noninteractive,
    ] {
        widget.corner_radius = 10.into();
        widget.fg_stroke.color = p.text;
    }
    style.spacing.item_spacing = Vec2::new(9.0, 10.0);
    style.spacing.button_padding = Vec2::new(14.0, 10.0);
    style.animation_time = if prefs.reduced_motion { 0.0 } else { 0.16 };
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, FontId::proportional(25.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, FontId::proportional(11.0));
    ctx.set_theme(theme);
    ctx.set_style_of(theme, style);
    p
}
fn card(p: Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(18.0)
        .inner_margin(18.0)
}
fn pill(ui: &mut egui::Ui, text: impl Into<String>, fill: Color32, ink: Color32) {
    egui::Frame::new()
        .fill(fill)
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(text.into()).color(ink).size(11.0));
        });
}

impl PopyApp {
    pub(super) fn start_setup(&mut self) {
        if self.studio.setup.is_some() {
            return;
        }
        if self.active.is_some() || self.queue_enabled {
            self.studio.notice = Some(("update_busy_hint", Instant::now()));
            return;
        }
        let result = (|| -> anyhow::Result<ManagedChild> {
            let directory = self.root.join(".tools");
            fs::create_dir_all(&directory)?;
            let powershell = std::env::var_os("SystemRoot")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("C:\\Windows"))
                .join("System32/WindowsPowerShell/v1.0/powershell.exe");
            let mut command = Command::new(powershell);
            command
                // A GUI launched from PowerShell 7 can inherit its module path.
                // Let Windows PowerShell initialize its own compatible modules.
                .env_remove("PSModulePath")
                .current_dir(&self.root)
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                ])
                .arg(self.root.join("scripts/bootstrap.ps1"))
                .arg("-SkipRust")
                .stdout(File::create(directory.join("setup.stdout.log"))?)
                .stderr(File::create(directory.join("setup.stderr.log"))?);
            ManagedChild::spawn(&mut command)
        })();
        match result {
            Ok(child) => {
                self.studio.setup = Some(child);
                self.studio.setup_state = "downloading";
                self.runtime_ready = false;
                self.message.clear();
            }
            Err(error) => {
                self.studio.setup_state = "failed";
                self.message = format!("{error:#}");
            }
        }
    }
    fn poll_setup(&mut self) {
        let done = self
            .studio
            .setup
            .as_mut()
            .and_then(|child| match child.child.try_wait() {
                Ok(Some(status)) => Some(status.success()),
                Err(_) => Some(false),
                _ => None,
            });
        if let Some(success) = done {
            self.studio.setup.take();
            self.runtime_ready = success && Tools::load(&self.root).is_ok();
            self.studio.setup_state = if self.runtime_ready {
                "completed"
            } else {
                "failed"
            };
            if self.runtime_ready {
                self.studio.notice = Some(("ready", Instant::now()));
                self.message.clear();
            } else {
                self.message = fs::read(self.root.join(".tools/setup.stderr.log"))
                    .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                    .unwrap_or_else(|_| "Runtime setup failed.".into());
            }
        }
    }
    pub(super) fn restore_update_result(&mut self) {
        let result = fs::read(self.root.join(".tools/update-result.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
        if let Some(result) = result {
            let id = result["id"].as_str().unwrap_or_default();
            if !id.is_empty() && id != self.session.preferences.update_result_seen {
                self.session.preferences.update_result_seen = id.into();
                if result["state"] == "completed" {
                    self.studio.notice = Some(("updated", Instant::now()));
                } else {
                    self.studio.settings_open = true;
                    self.studio.tab = 2;
                    self.studio.update.status = "update_error";
                    self.studio.update.error =
                        result["error"].as_str().unwrap_or("Update failed").into();
                }
                self.save_session();
            }
        }
    }
    fn l(&self, key: &str) -> String {
        i18n::text(&self.session.preferences.locale, key)
    }
    fn action_button(
        &self,
        ui: &mut egui::Ui,
        action: Action,
        primary: bool,
        enabled: bool,
        p: Palette,
    ) -> bool {
        let label = self.l(action.label());
        let button = egui::Button::new(RichText::new(label).color(if primary {
            Color32::WHITE
        } else {
            p.text
        }))
        .fill(if primary { p.accent } else { p.soft })
        .corner_radius(12.0)
        .min_size(Vec2::new(0.0, 40.0));
        ui.add_enabled(enabled, button)
            .on_hover_text(self.session.preferences.shortcuts.label(action))
            .clicked()
    }
    fn icon_action(&self, ui: &mut egui::Ui, action: Action, enabled: bool, p: Palette) -> bool {
        let response = ui
            .add_enabled(
                enabled,
                egui::Button::new("").min_size(Vec2::new(35.0, 32.0)),
            )
            .on_hover_text(format!(
                "{} · {}",
                self.l(action.label()),
                self.session.preferences.shortcuts.label(action)
            ));
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::Role::Button, enabled, self.l(action.label()))
        });
        let c = response.rect.center();
        let stroke = Stroke::new(
            1.3,
            if enabled {
                p.text
            } else {
                p.muted.gamma_multiply(0.5)
            },
        );
        if matches!(action, Action::MoveUp | Action::MoveDown) {
            let direction = if action == Action::MoveUp { -1.0 } else { 1.0 };
            ui.painter().line_segment(
                [
                    c + Vec2::new(0.0, 5.0 * direction),
                    c - Vec2::new(0.0, 5.0 * direction),
                ],
                stroke,
            );
            ui.painter().line_segment(
                [
                    c + Vec2::new(-4.0, direction),
                    c + Vec2::new(0.0, 5.0 * direction),
                ],
                stroke,
            );
            ui.painter().line_segment(
                [
                    c + Vec2::new(4.0, direction),
                    c + Vec2::new(0.0, 5.0 * direction),
                ],
                stroke,
            );
        } else {
            ui.painter().rect_stroke(
                Rect::from_center_size(c + Vec2::new(0.0, 1.0), Vec2::new(8.0, 9.0)),
                1.5,
                stroke,
                egui::StrokeKind::Inside,
            );
            ui.painter().line_segment(
                [c + Vec2::new(-6.0, -5.0), c + Vec2::new(6.0, -5.0)],
                stroke,
            );
        }
        response.clicked()
    }
    fn perform(&mut self, action: Action, ctx: &egui::Context) {
        use Action::*;
        match action {
            Setup => self.start_setup(),
            Add => {
                if let Some(files) = rfd::FileDialog::new()
                    .set_title(self.l("add"))
                    .add_filter("Video", &["mp4", "mkv", "mov"])
                    .pick_files()
                {
                    self.add_files(files);
                }
            }
            Preview => {
                if self.runtime_ready
                    && self.studio.setup.is_none()
                    && self.selected.is_some()
                    && !self.studio.update.auto_install
                {
                    self.schedule(true);
                }
            }
            Start => {
                if self.runtime_ready
                    && self.studio.setup.is_none()
                    && !self.studio.update.auto_install
                {
                    self.schedule(false);
                }
            }
            Stop => {
                if self.studio.setup.take().is_some() {
                    self.studio.setup_state = "cancelled";
                    self.runtime_ready = false;
                }
                self.cancel();
                self.studio.notice = Some(("cancelled_hint", Instant::now()));
            }
            ChooseOutput => {
                if let Some(folder) = rfd::FileDialog::new()
                    .set_title(self.l("choose_folder"))
                    .pick_folder()
                {
                    self.session.settings.output = folder;
                    self.save_session();
                }
            }
            OpenVideo => {
                if let Some(entry) = self
                    .selected
                    .and_then(|i| self.session.entries.get(i))
                    .filter(|e| e.state == "completed")
                {
                    open_path(&entry.output.join("enhanced.mp4"));
                }
            }
            OpenFolder => {
                if let Some(entry) = self
                    .selected
                    .and_then(|i| self.session.entries.get(i))
                    .filter(|e| e.output.is_dir())
                {
                    open_path(&entry.output);
                }
            }
            Retry => {
                if let Some(entry) = self
                    .selected
                    .and_then(|i| self.session.entries.get(i))
                    .filter(|e| matches!(e.state.as_str(), "failed" | "cancelled" | "interrupted"))
                {
                    let mut entry = entry.clone();
                    entry.settings = self.session.settings.clone();
                    entry.state = "queued".into();
                    entry.output = PathBuf::new();
                    entry.error.clear();
                    entry.progress = None;
                    self.session.entries.push(entry);
                    self.selected = Some(self.session.entries.len() - 1);
                    self.queue_enabled = true;
                    self.save_session();
                }
            }
            Remove => {
                if let Some(index) = self
                    .selected
                    .filter(|&i| self.active.as_ref().is_none_or(|a| a.index != i))
                {
                    self.session.entries.remove(index);
                    if let Some(active) = &mut self.active
                        && active.index > index
                    {
                        active.index -= 1;
                    }
                    self.selected = if self.session.entries.is_empty() {
                        None
                    } else {
                        Some(index.min(self.session.entries.len() - 1))
                    };
                    self.save_session();
                }
            }
            MoveUp | MoveDown => {
                if let Some(index) = self.selected {
                    let other = if action == MoveUp {
                        index.checked_sub(1)
                    } else {
                        index
                            .checked_add(1)
                            .filter(|&i| i < self.session.entries.len())
                    };
                    if let Some(other) = other
                        && self
                            .active
                            .as_ref()
                            .is_none_or(|a| a.index != index && a.index != other)
                    {
                        self.session.entries.swap(index, other);
                        self.selected = Some(other);
                        self.save_session();
                    }
                }
            }
            Previous | Next => {
                if !self.session.entries.is_empty() {
                    let count = self.session.entries.len();
                    let index = self.selected.unwrap_or(0);
                    self.selected = Some(if action == Previous {
                        (index + count - 1) % count
                    } else {
                        (index + 1) % count
                    });
                }
            }
            Color => {
                self.session.settings.auto_color = !self.session.settings.auto_color;
                self.save_session();
            }
            General | Anime => {
                self.session.settings.model = if action == General {
                    Model::General
                } else {
                    Model::Anime
                };
                self.save_session();
            }
            Settings => {
                self.studio.settings_open = !self.studio.settings_open;
                self.studio.tab = 0;
                if !self.studio.settings_open {
                    self.studio.capture = None;
                }
            }
            Help => {
                self.studio.settings_open = true;
                self.studio.tab = 1;
            }
            Theme => {
                self.session.preferences.dark = !self.session.preferences.dark;
                self.save_session();
            }
            Motion => {
                self.session.preferences.reduced_motion = !self.session.preferences.reduced_motion;
                self.save_session();
            }
            Companion => {
                self.session.preferences.companion = (self.session.preferences.companion + 1) % 3;
                self.save_session();
            }
            Before => self.split = 1.0,
            After => self.split = 0.0,
            Fit => self.studio.zoom = 1.0,
            ZoomIn => self.studio.zoom = (self.studio.zoom + 0.25).min(4.0),
            ZoomOut => self.studio.zoom = (self.studio.zoom - 0.25).max(1.0),
            Updates => {
                self.studio.settings_open = true;
                self.studio.tab = 2;
                self.check_updates();
            }
            Fullscreen => {
                self.studio.fullscreen = !self.studio.fullscreen;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.studio.fullscreen));
            }
        }
    }
    fn check_updates(&mut self) {
        if matches!(self.studio.update.status, "checking" | "downloading") {
            return;
        }
        let (send, receive) = mpsc::channel();
        self.studio.update.receiver = Some(receive);
        self.studio.update.status = "checking";
        self.studio.update.error.clear();
        let previews = self.session.preferences.preview_updates;
        std::thread::spawn(move || {
            let result = updater::check(previews).map_err(|e| format!("{e:#}"));
            let _ = send.send(UpdateEvent::Checked(result));
        });
    }
    fn download_update(&mut self) {
        let Some(manifest) = self.studio.update.manifest.clone() else {
            return;
        };
        if self.active.is_some() || self.queue_enabled || self.studio.setup.is_some() {
            return;
        }
        let (send, receive) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.studio.update.cancel = Arc::clone(&cancel);
        self.studio.update.receiver = Some(receive);
        self.studio.update.status = "downloading";
        self.studio.update.auto_install = true;
        self.studio.update.fraction = 0.0;
        let root = self.root.clone();
        std::thread::spawn(move || {
            let progress = send.clone();
            let result = updater::download(&root, &manifest, &cancel, |f| {
                let _ = progress.send(UpdateEvent::Progress(f));
            })
            .map_err(|e| format!("{e:#}"));
            let _ = send.send(UpdateEvent::Downloaded(result));
        });
    }
    fn poll_updates(&mut self, ctx: &egui::Context) {
        let events: Vec<_> = self
            .studio
            .update
            .receiver
            .as_ref()
            .map(|r| r.try_iter().collect())
            .unwrap_or_default();
        for event in events {
            match event {
                UpdateEvent::Progress(f) => self.studio.update.fraction = f,
                UpdateEvent::Checked(Ok(Some(manifest))) => {
                    self.studio.update.manifest = Some(manifest);
                    self.studio.update.status = "update_available";
                }
                UpdateEvent::Checked(Ok(None)) => self.studio.update.status = "up_to_date",
                UpdateEvent::Downloaded(Ok(staged)) => {
                    self.studio.update.staged = Some(staged);
                    self.studio.update.status = "update_ready";
                }
                UpdateEvent::Checked(Err(error)) => {
                    self.studio.update.status = "update_error";
                    self.studio.update.error = error;
                    self.studio.update.auto_install = false;
                }
                UpdateEvent::Downloaded(Err(error)) => {
                    self.studio.update.status = if self.studio.update.cancel.load(Ordering::Relaxed)
                    {
                        "cancelled"
                    } else {
                        "update_error"
                    };
                    self.studio.update.error = error;
                    self.studio.update.auto_install = false;
                }
            }
        }
        if self.studio.update_test.is_some() && self.studio.update.status == "update_available" {
            self.download_update();
        }
        if let Some(path) = &self.studio.update_test
            && matches!(self.studio.update.status, "update_error" | "up_to_date")
        {
            let _ = popy::pipeline::write_json(
                path,
                &serde_json::json!({"state":self.studio.update.status,"error":self.studio.update.error}),
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.studio.update.auto_install
            && self.studio.update.status == "update_ready"
            && self.active.is_none()
            && !self.queue_enabled
            && self.studio.setup.is_none()
        {
            if let Some(staged) = &self.studio.update.staged {
                match updater::install(&self.root, staged) {
                    Ok(()) => {
                        if let Some(path) = &self.studio.update_test {
                            let _ = popy::pipeline::write_json(
                                path,
                                &serde_json::json!({"state":"installer_started"}),
                            );
                        }
                        self.save_session();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    Err(error) => {
                        self.studio.update.error = format!("{error:#}");
                        self.studio.update.status = "update_error";
                    }
                }
            }
            self.studio.update.auto_install = false;
        }
    }
    fn keyboard(&mut self, ctx: &egui::Context) {
        if let Some(action) = self.studio.capture {
            let event = ctx.input(|i| {
                i.events.iter().find_map(|e| {
                    if let egui::Event::Key {
                        key,
                        pressed: true,
                        repeat: false,
                        modifiers,
                        ..
                    } = e
                    {
                        Some((*key, *modifiers))
                    } else {
                        None
                    }
                })
            });
            if let Some((key, modifiers)) = event {
                ctx.input_mut(|i| {
                    i.consume_key(modifiers, key);
                });
                if key == egui::Key::Escape {
                    self.studio.capture = None;
                    self.studio.conflict = None;
                } else if key != egui::Key::Tab {
                    match self
                        .session
                        .preferences
                        .shortcuts
                        .rebind(Binding::from_event(action, key, modifiers))
                    {
                        Ok(()) => {
                            self.studio.capture = None;
                            self.studio.conflict = None;
                            self.save_session();
                        }
                        Err(conflict) => self.studio.conflict = Some(conflict),
                    }
                }
            }
        } else {
            for action in self.session.preferences.shortcuts.consume(ctx) {
                self.perform(action, ctx);
            }
        }
    }
    fn title_bar(&mut self, ui: &mut egui::Ui, p: Palette) {
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), 44.0),
            Sense::click_and_drag(),
        );
        if response.drag_started() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        if response.double_clicked() {
            let maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
        }
        let painter = ui.painter();
        if let Some(logo) = &self.logo {
            painter.image(
                logo.id(),
                Rect::from_min_size(rect.min + Vec2::new(14.0, 5.0), Vec2::splat(32.0)),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        painter.text(
            rect.min + Vec2::new(53.0, 22.0),
            egui::Align2::LEFT_CENTER,
            "popy",
            FontId::proportional(20.0),
            p.text,
        );
        painter.text(
            rect.min + Vec2::new(112.0, 22.0),
            egui::Align2::LEFT_CENTER,
            self.l("app_tagline"),
            FontId::proportional(11.0),
            p.muted,
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(rect.shrink2(Vec2::new(10.0, 6.0)))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                for button in 0..3 {
                    let (r, response) =
                        ui.allocate_exact_size(Vec2::new(36.0, 30.0), Sense::click());
                    let hover = ui.ctx().animate_bool_with_time(
                        response.id,
                        response.hovered(),
                        if self.session.preferences.reduced_motion {
                            0.0
                        } else {
                            0.13
                        },
                    );
                    if hover > 0.0 {
                        ui.painter().rect_filled(
                            r,
                            9.0,
                            if button == 0 {
                                p.bad.gamma_multiply(hover * 0.15)
                            } else {
                                p.soft.gamma_multiply(hover)
                            },
                        );
                    }
                    let c = r.center();
                    let stroke = Stroke::new(1.3, p.text);
                    match button {
                        0 => {
                            ui.painter().line_segment(
                                [c + Vec2::splat(-4.0), c + Vec2::splat(4.0)],
                                stroke,
                            );
                            ui.painter().line_segment(
                                [c + Vec2::new(-4.0, 4.0), c + Vec2::new(4.0, -4.0)],
                                stroke,
                            );
                        }
                        1 => {
                            ui.painter().rect_stroke(
                                Rect::from_center_size(c, Vec2::splat(9.0)),
                                1.5,
                                stroke,
                                egui::StrokeKind::Inside,
                            );
                        }
                        _ => {
                            ui.painter().line_segment(
                                [c + Vec2::new(-5.0, 2.0), c + Vec2::new(5.0, 2.0)],
                                stroke,
                            );
                        }
                    }
                    if response.clicked() {
                        match button {
                            0 => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
                            1 => {
                                let maximized =
                                    ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(
                                    !maximized,
                                ));
                            }
                            _ => ui
                                .ctx()
                                .send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                        }
                    }
                }
            },
        );
    }

    fn library(&mut self, ui: &mut egui::Ui, p: Palette) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(self.l("queue")).strong().size(16.0));
            pill(ui, self.session.entries.len().to_string(), p.soft, p.muted);
        });
        ui.add_space(5.0);
        if self.action_button(ui, Action::Add, true, true, p) {
            self.perform(Action::Add, ui.ctx());
        }
        ui.add_space(12.0);
        let height = (ui.available_height() - 152.0).max(120.0);
        let mut clicked = None;
        let mut action = None;
        egui::ScrollArea::vertical()
            .id_salt("video-library")
            .max_height(height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.session.entries.is_empty() {
                    ui.add_space(30.0);
                    ui.label(RichText::new(self.l("empty_queue")).color(p.muted));
                }
                for (index, entry) in self.session.entries.iter().enumerate() {
                    let selected = self.selected == Some(index);
                    let status = if entry.state == "ready" {
                        "ready"
                    } else if entry.state == "queued" {
                        "queued"
                    } else if entry.state == "running" {
                        "running"
                    } else if entry.state == "completed" {
                        "completed"
                    } else if entry.state == "cancelled" {
                        "cancelled"
                    } else if entry.state == "interrupted" {
                        "interrupted"
                    } else {
                        "failed"
                    };
                    let frame = egui::Frame::new()
                        .fill(if selected { p.tint } else { p.surface })
                        .corner_radius(13.0)
                        .stroke(Stroke::new(
                            1.0,
                            if selected {
                                p.accent.gamma_multiply(0.45)
                            } else {
                                p.line
                            },
                        ))
                        .inner_margin(12.0);
                    let shown = frame.show(ui, |ui| {
                        ui.set_min_width((ui.available_width() - 2.0).max(80.0));
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{:02}", index + 1))
                                    .size(11.0)
                                    .color(p.muted),
                            );
                            ui.add(
                                egui::Label::new(
                                    RichText::new(filename(&entry.input)).strong().size(12.0),
                                )
                                .truncate(),
                            )
                            .on_hover_text(entry.input.display().to_string());
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(self.l(status)).size(11.0).color(
                                if status == "completed" {
                                    p.good
                                } else if status == "failed" {
                                    p.bad
                                } else {
                                    p.muted
                                },
                            ));
                            if entry.preview {
                                ui.label(
                                    RichText::new(self.l("preview")).size(10.0).color(p.accent),
                                );
                            }
                        });
                    });
                    let response = shown.response.interact(Sense::click());
                    if response.clicked() {
                        clicked = Some(index);
                    }
                    response.context_menu(|ui| {
                        for command in [
                            Action::OpenVideo,
                            Action::OpenFolder,
                            Action::Retry,
                            Action::MoveUp,
                            Action::MoveDown,
                            Action::Remove,
                        ] {
                            if ui.button(self.l(command.label())).clicked() {
                                clicked = Some(index);
                                action = Some(command);
                                ui.close();
                            }
                        }
                    });
                    ui.add_space(3.0);
                }
            });
        if let Some(index) = clicked {
            self.selected = Some(index);
        }
        if let Some(action) = action {
            self.perform(action, ui.ctx());
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            for action in [Action::MoveUp, Action::MoveDown, Action::Remove] {
                if self.icon_action(ui, action, self.selected.is_some(), p) {
                    self.perform(action, ui.ctx());
                }
            }
        });
        if ui
            .add_enabled(
                self.active.is_none(),
                egui::Button::new(self.l("clear_completed")).frame(false),
            )
            .clicked()
        {
            self.session.entries.retain(|e| {
                !matches!(
                    e.state.as_str(),
                    "completed" | "failed" | "cancelled" | "interrupted"
                )
            });
            self.selected = None;
            self.save_session();
        }
        ui.add_space(8.0);
        ui.label(
            RichText::new(self.l("private_hint"))
                .size(10.0)
                .color(p.muted),
        );
        ui.label(
            RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                .size(10.0)
                .color(p.muted),
        );
    }

    fn inspector(&mut self, ui: &mut egui::Ui, p: Palette) {
        let lang = self.session.preferences.locale.clone();
        let l = |key| i18n::text(&lang, key);
        egui::ScrollArea::vertical()
            .id_salt("inspector")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(l("enhancement")).strong().size(17.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(l("settings")).clicked() {
                            self.perform(Action::Settings, ui.ctx());
                        }
                    });
                });
                ui.add_space(10.0);
                let mut options = self.session.settings.clone();
                let mut changed = false;
                card(p).show(ui, |ui| {
                    ui.label(RichText::new(l("model")).color(p.muted).size(11.0));
                    changed |= ui
                        .selectable_value(&mut options.model, Model::General, l("model_general"))
                        .changed();
                    changed |= ui
                        .selectable_value(&mut options.model, Model::Anime, l("model_anime"))
                        .changed();
                    ui.add_space(3.0);
                    ui.label(
                        RichText::new(l(if options.model == Model::General {
                            "heavy_model_hint"
                        } else {
                            "anime_hint"
                        }))
                        .color(p.muted)
                        .size(11.0),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} ≤ {}",
                            l("source"),
                            if options.model == Model::General {
                                "960×540"
                            } else {
                                "1920×1080"
                            }
                        ))
                        .size(10.0)
                        .color(p.muted),
                    );
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("2×").size(29.0).color(p.accent).strong());
                        ui.label(RichText::new(l("result")).color(p.muted));
                    });
                    ui.add_space(12.0);
                    changed |= ui.checkbox(&mut options.auto_color, l("color")).changed();
                    ui.label(RichText::new(l("color_hint")).color(p.muted).size(11.0));
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.label(l("preview_length"));
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.session.preferences.preview_seconds)
                                    .range(1..=10)
                                    .suffix(" s"),
                            )
                            .changed()
                        {
                            self.save_session();
                        }
                    });
                });
                ui.add_space(9.0);
                egui::CollapsingHeader::new(l("advanced")).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(l("gpu"));
                        changed |= ui
                            .add(egui::DragValue::new(&mut options.gpu).range(0..=15))
                            .changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label(l("tile"));
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut options.tile)
                                    .range(32..=256)
                                    .speed(8),
                            )
                            .changed();
                    });
                });
                ui.add_space(8.0);
                ui.label(RichText::new(l("output")).color(p.muted).size(11.0));
                let folder = &options.output;
                ui.add(
                    egui::Label::new(
                        RichText::new(format!(
                            "… / {} / {}",
                            folder.parent().map(filename).unwrap_or_default(),
                            filename(folder)
                        ))
                        .size(11.0),
                    )
                    .truncate(),
                )
                .on_hover_text(folder.display().to_string());
                if self.action_button(ui, Action::ChooseOutput, false, true, p) {
                    self.perform(Action::ChooseOutput, ui.ctx());
                    options.output = self.session.settings.output.clone();
                }
                if changed {
                    self.session.settings = options;
                    self.save_session();
                }
                ui.add_space(7.0);
                ui.label(RichText::new(l("new_jobs_hint")).color(p.muted).size(10.0));
                ui.add_space(18.0);
                self.buddy(ui, p);
            });
    }

    fn buddy(&mut self, ui: &mut egui::Ui, p: Palette) {
        let kind = self.session.preferences.companion;
        if kind == 2 {
            return;
        }
        let texture = if kind == 0 {
            self.studio.fox.as_ref()
        } else {
            self.studio.bear.as_ref()
        };
        let Some(texture) = texture else {
            return;
        };
        let width = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 120.0), Sense::click());
        let time = self.studio.started.elapsed().as_secs_f32();
        let motion = if self.session.preferences.reduced_motion {
            0.0
        } else {
            1.0
        };
        let hover = ui
            .ctx()
            .animate_bool_with_time(response.id, response.hovered(), 0.18)
            * motion;
        let bounce = self
            .studio
            .completed
            .map(|t| {
                let s = t.elapsed().as_secs_f32();
                if s < 1.4 {
                    (s * 15.0).sin().abs() * (1.0 - s / 1.4) * 10.0
                } else {
                    0.0
                }
            })
            .unwrap_or(0.0)
            * motion;
        let bob = (time * 1.8).sin() * 2.0 * motion - hover * 4.0 - bounce;
        let size = Vec2::new(112.0, 112.0 + (time * 1.8).sin() * 1.2 * motion);
        let center = rect.center() + Vec2::new(0.0, bob);
        let shadow: Vec<_> = (0..24)
            .map(|i| {
                let a = i as f32 / 24.0 * std::f32::consts::TAU;
                rect.center() + Vec2::new(a.cos() * 38.0, 58.0 + a.sin() * 5.0)
            })
            .collect();
        ui.painter().add(egui::Shape::convex_polygon(
            shadow,
            p.muted.gamma_multiply(0.10),
            Stroke::NONE,
        ));
        ui.painter().image(
            texture.id(),
            Rect::from_center_size(center, size),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        if response.clicked() {
            self.perform(Action::Companion, ui.ctx());
        }
        let status = if self.studio.setup.is_some() {
            "downloading"
        } else if self.active.is_some() {
            "status_working"
        } else if self
            .selected
            .and_then(|i| self.session.entries.get(i))
            .is_some_and(|e| e.state == "completed")
        {
            "status_done"
        } else if self
            .selected
            .and_then(|i| self.session.entries.get(i))
            .is_some_and(|e| e.state == "failed")
        {
            "status_error"
        } else {
            "status_idle"
        };
        egui::Frame::new()
            .fill(p.tint)
            .corner_radius(12.0)
            .inner_margin(11.0)
            .show(ui, |ui| {
                ui.label(RichText::new(self.l(status)).size(12.0).color(p.text));
            });
    }

    fn canvas(&mut self, ui: &mut egui::Ui, p: Palette) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(self.l("workspace")).strong().size(25.0));
                if let Some(entry) = self.selected.and_then(|i| self.session.entries.get(i)) {
                    ui.add(
                        egui::Label::new(
                            RichText::new(filename(&entry.input))
                                .color(p.muted)
                                .size(12.0),
                        )
                        .truncate(),
                    );
                } else {
                    ui.label(RichText::new(self.l("app_tagline")).color(p.muted));
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.action_button(ui, Action::Help, false, true, p) {
                    self.perform(Action::Help, ui.ctx());
                }
                if self.action_button(ui, Action::Theme, false, true, p) {
                    self.perform(Action::Theme, ui.ctx());
                }
            });
        });
        ui.add_space(14.0);
        let height = (ui.available_height() - 208.0).clamp(260.0, 560.0);
        let had_comparison = self.comparison.is_some();
        card(p).inner_margin(14.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                pill(ui, self.l("source"), p.soft, p.muted);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    pill(
                        ui,
                        format!("Popy · 2×  {}", self.l("result")),
                        p.tint,
                        p.accent,
                    );
                });
            });
            ui.add_space(7.0);
            egui::Frame::new()
                .fill(p.canvas)
                .corner_radius(13.0)
                .show(ui, |ui| {
                    if let Some(texture) = &self.comparison {
                        let size = texture.size();
                        let ratio = (size[0] as f32 / 2.0) / size[1] as f32;
                        let viewport = Vec2::new(ui.available_width(), height);
                        let fit = Vec2::new(
                            viewport.x.min(viewport.y * ratio),
                            viewport.y.min(viewport.x / ratio),
                        );
                        let desired = fit * self.studio.zoom;
                        let held = ui.ctx().input(|i| i.key_down(egui::Key::Space))
                            && !ui.ctx().text_edit_focused()
                            && !self.studio.settings_open;
                        let split = if held { 1.0 } else { self.split };
                        egui::ScrollArea::both()
                            .id_salt("comparison-pan")
                            .max_height(height)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.set_min_size(viewport);
                                let offset = Vec2::new(
                                    (viewport.x - desired.x).max(0.0) / 2.0,
                                    (viewport.y - desired.y).max(0.0) / 2.0,
                                );
                                let (space, _) = ui.allocate_exact_size(
                                    Vec2::new(viewport.x.max(desired.x), viewport.y.max(desired.y)),
                                    Sense::hover(),
                                );
                                let rect = Rect::from_min_size(space.min + offset, desired);
                                let response = ui.interact(
                                    rect,
                                    egui::Id::unique("image-divider"),
                                    Sense::click_and_drag(),
                                );
                                ui.painter().image(
                                    texture.id(),
                                    rect,
                                    Rect::from_min_max(Pos2::new(0.5, 0.0), Pos2::new(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                                let x = rect.left() + rect.width() * split;
                                ui.painter()
                                    .with_clip_rect(Rect::from_min_max(
                                        rect.min,
                                        Pos2::new(x, rect.bottom()),
                                    ))
                                    .image(
                                        texture.id(),
                                        rect,
                                        Rect::from_min_max(Pos2::ZERO, Pos2::new(0.5, 1.0)),
                                        Color32::WHITE,
                                    );
                                ui.painter().line_segment(
                                    [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                                    Stroke::new(2.0, p.accent),
                                );
                                ui.painter().circle_filled(
                                    Pos2::new(x, rect.center().y),
                                    12.0,
                                    p.surface,
                                );
                                ui.painter().line_segment(
                                    [
                                        Pos2::new(x - 3.0, rect.center().y - 4.0),
                                        Pos2::new(x - 3.0, rect.center().y + 4.0),
                                    ],
                                    Stroke::new(1.0, p.accent),
                                );
                                ui.painter().line_segment(
                                    [
                                        Pos2::new(x + 3.0, rect.center().y - 4.0),
                                        Pos2::new(x + 3.0, rect.center().y + 4.0),
                                    ],
                                    Stroke::new(1.0, p.accent),
                                );
                                if (response.dragged() || response.clicked())
                                    && let Some(pointer) = response.interact_pointer_pos()
                                {
                                    self.split =
                                        ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                                }
                            });
                    } else {
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), height),
                            Sense::click(),
                        );
                        let center = rect.center();
                        let frame = Rect::from_center_size(
                            center - Vec2::new(0.0, 42.0),
                            Vec2::new(86.0, 64.0),
                        );
                        ui.painter().rect_filled(frame, 16.0, p.tint);
                        ui.painter().rect_stroke(
                            frame,
                            16.0,
                            Stroke::new(1.5, p.accent.gamma_multiply(0.5)),
                            egui::StrokeKind::Inside,
                        );
                        for side in [-1.0, 1.0] {
                            for row in [-1.0, 0.0, 1.0] {
                                ui.painter().rect_filled(
                                    Rect::from_center_size(
                                        frame.center() + Vec2::new(side * 30.0, row * 17.0),
                                        Vec2::new(8.0, 6.0),
                                    ),
                                    2.0,
                                    p.accent.gamma_multiply(0.35),
                                );
                            }
                        }
                        ui.painter().text(
                            center + Vec2::new(0.0, 22.0),
                            egui::Align2::CENTER_CENTER,
                            self.l("drop_title"),
                            FontId::proportional(21.0),
                            p.text,
                        );
                        ui.painter().text(
                            center + Vec2::new(0.0, 53.0),
                            egui::Align2::CENTER_CENTER,
                            self.l("drop_hint"),
                            FontId::proportional(12.0),
                            p.muted,
                        );
                        ui.painter().text(
                            center + Vec2::new(0.0, 86.0),
                            egui::Align2::CENTER_CENTER,
                            self.session.preferences.shortcuts.label(Action::Add),
                            FontId::monospace(11.0),
                            p.accent,
                        );
                        if response.clicked() {
                            self.perform(Action::Add, ui.ctx());
                        }
                    }
                });
            if had_comparison {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().slider_width =
                        (ui.available_width() * 0.42).clamp(90.0, 300.0);
                    ui.add(egui::Slider::new(&mut self.split, 0.0..=1.0).show_value(false));
                    if ui.small_button(self.l("fit")).clicked() {
                        self.studio.zoom = 1.0;
                    }
                    if ui
                        .small_button("−")
                        .on_hover_text(self.l("zoom_out"))
                        .clicked()
                    {
                        self.studio.zoom = (self.studio.zoom - 0.25).max(1.0);
                    }
                    ui.label(
                        RichText::new(format!("{:.0}%", self.studio.zoom * 100.0))
                            .size(11.0)
                            .color(p.muted),
                    );
                    if ui
                        .small_button("+")
                        .on_hover_text(self.l("zoom_in"))
                        .clicked()
                    {
                        self.studio.zoom = (self.studio.zoom + 0.25).min(4.0);
                    }
                });
            }
        });
        ui.add_space(9.0);
        ui.horizontal_wrapped(|ui| {
            let ready = self
                .session
                .entries
                .iter()
                .any(|e| matches!(e.state.as_str(), "ready" | "queued"));
            if self.action_button(
                ui,
                Action::Preview,
                false,
                self.runtime_ready
                    && self.studio.setup.is_none()
                    && self.selected.is_some()
                    && !self.studio.update.auto_install,
                p,
            ) {
                self.perform(Action::Preview, ui.ctx());
            }
            if self.action_button(
                ui,
                Action::Start,
                true,
                self.runtime_ready
                    && self.studio.setup.is_none()
                    && ready
                    && !self.studio.update.auto_install,
                p,
            ) {
                self.perform(Action::Start, ui.ctx());
            }
            if self.action_button(
                ui,
                Action::Stop,
                false,
                self.active.is_some() || self.queue_enabled || self.studio.setup.is_some(),
                p,
            ) {
                self.perform(Action::Stop, ui.ctx());
            }
        });
        ui.add_space(7.0);
        if let Some(active) = &self.active {
            let progress = self.session.entries[active.index].progress.as_ref();
            let fraction = progress
                .filter(|p| p.total_frames > 0)
                .map(|p| p.completed_frames as f32 / p.total_frames as f32 * 0.92)
                .unwrap_or(0.0);
            let smooth = ui.ctx().animate_value_with_time(
                egui::Id::unique("work-progress"),
                fraction,
                if self.session.preferences.reduced_motion {
                    0.0
                } else {
                    0.25
                },
            );
            let stage = progress
                .map(|p| match p.stage.as_str() {
                    "inspect" | "prepare" => "inspect",
                    "decode" => "decode",
                    "upscale" => "upscale",
                    "segment" | "encode" => "encode",
                    _ => "validate",
                })
                .unwrap_or("inspect");
            ui.add(
                egui::ProgressBar::new(smooth)
                    .fill(p.accent)
                    .text(self.l(stage))
                    .animate(!self.session.preferences.reduced_motion),
            );
            ui.label(
                RichText::new(format!(
                    "{}  ·  {:.0} s",
                    filename(&self.session.entries[active.index].input),
                    active.start.elapsed().as_secs_f64()
                ))
                .size(11.0)
                .color(p.muted),
            );
        } else if let Some(entry) = self
            .selected
            .and_then(|i| self.session.entries.get(i))
            .cloned()
        {
            if entry.state == "completed" {
                ui.horizontal_wrapped(|ui| {
                    if self.action_button(ui, Action::OpenVideo, false, true, p) {
                        self.perform(Action::OpenVideo, ui.ctx());
                    }
                    if self.action_button(ui, Action::OpenFolder, false, true, p) {
                        self.perform(Action::OpenFolder, ui.ctx());
                    }
                });
            }
            if !entry.error.is_empty() {
                ui.horizontal(|ui| {
                    ui.colored_label(p.bad, self.l("failed"));
                    if ui.button(self.l("retry")).clicked() {
                        self.perform(Action::Retry, ui.ctx());
                    }
                });
                egui::CollapsingHeader::new(self.l("show_details")).show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(65.0)
                        .show(ui, |ui| {
                            ui.small(&entry.error);
                        });
                });
            }
        }
        if let Some((notice, time)) = self.studio.notice
            && time.elapsed() < Duration::from_secs(6)
        {
            ui.label(RichText::new(self.l(notice)).color(p.muted).size(11.0));
        }
        ui.add_space(6.0);
        ui.label(
            RichText::new(self.l(if had_comparison {
                "compare_hint"
            } else {
                "format_hint"
            }))
            .size(10.0)
            .color(p.muted),
        );
    }

    fn preferences_window(&mut self, ctx: &egui::Context, p: Palette) {
        if !self.studio.settings_open {
            return;
        }
        let mut open = true;
        let title = self.l("settings");
        let lang = self.session.preferences.locale.clone();
        let l = |key| i18n::text(&lang, key);
        egui::Window::new(title)
            .id(egui::Id::unique("preferences-window"))
            .open(&mut open)
            .collapsible(false)
            .default_width(650.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (tab, key) in [(0, "appearance"), (1, "shortcuts"), (2, "updates")] {
                        if ui
                            .selectable_label(self.studio.tab == tab, l(key))
                            .clicked()
                        {
                            self.studio.tab = tab;
                        }
                    }
                });
                ui.add_space(12.0);
                egui::ScrollArea::vertical()
                    .id_salt("settings-scroll")
                    .max_height(610.0)
                    .show(ui, |ui| match self.studio.tab {
                        0 => {
                            let mut prefs = self.session.preferences.clone();
                            let mut changed = false;
                            card(p).show(ui, |ui| {
                                ui.label(RichText::new(l("language")).strong());
                                egui::ComboBox::from_id_salt("language")
                                    .width(280.0)
                                    .selected_text(
                                        i18n::LANGUAGES
                                            .iter()
                                            .find(|a| a.code == prefs.locale)
                                            .map(|a| i18n::visual_text(a.native, a.code == "ar"))
                                            .unwrap_or_else(|| "English".into()),
                                    )
                                    .show_ui(ui, |ui| {
                                        for language in i18n::LANGUAGES {
                                            changed |= ui
                                                .selectable_value(
                                                    &mut prefs.locale,
                                                    language.code.into(),
                                                    i18n::visual_text(
                                                        language.native,
                                                        language.code == "ar",
                                                    ),
                                                )
                                                .changed();
                                        }
                                    });
                                ui.add_space(12.0);
                                ui.label(RichText::new(l("theme")).strong());
                                ui.horizontal(|ui| {
                                    changed |= ui
                                        .selectable_value(&mut prefs.dark, false, l("light"))
                                        .changed();
                                    changed |= ui
                                        .selectable_value(&mut prefs.dark, true, l("dark"))
                                        .changed();
                                });
                                changed |= ui
                                    .checkbox(&mut prefs.reduced_motion, l("reduced_motion"))
                                    .changed();
                                ui.add_space(8.0);
                                ui.label(RichText::new(l("companion")).strong());
                                ui.horizontal(|ui| {
                                    for (kind, key) in [(0, "fox"), (1, "bear"), (2, "hidden")] {
                                        changed |= ui
                                            .selectable_value(&mut prefs.companion, kind, l(key))
                                            .changed();
                                    }
                                });
                                ui.add_space(8.0);
                                ui.label(l("ui_scale"));
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut prefs.ui_scale, 0.85..=1.2)
                                            .step_by(0.05)
                                            .custom_formatter(|value, _| {
                                                format!("{:.0}%", value * 100.0)
                                            }),
                                    )
                                    .changed();
                            });
                            if changed {
                                ctx.set_zoom_factor(prefs.ui_scale);
                                self.session.preferences = prefs;
                                self.save_session();
                            }
                            ui.add_space(16.0);
                            ui.label(RichText::new(l("about")).strong());
                            ui.label(format!("Popy {} · Windows x64", env!("CARGO_PKG_VERSION")));
                            ui.label(RichText::new(l("private_hint")).color(p.muted));
                            ui.hyperlink_to(
                                "Apache-2.0 · GitHub",
                                "https://github.com/boranseasonnew/Popy",
                            );
                        }
                        1 => {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(l("shortcuts")).strong());
                                if ui.button(l("reset_defaults")).clicked() {
                                    self.session.preferences.shortcuts = Default::default();
                                    self.studio.capture = None;
                                    self.save_session();
                                }
                            });
                            if let Some(action) = self.studio.capture {
                                card(p).fill(p.tint).show(ui, |ui| {
                                    ui.label(RichText::new(l("press_keys")).color(p.accent));
                                    ui.label(l(action.label()));
                                    if let Some(conflict) = self.studio.conflict {
                                        ui.colored_label(
                                            p.bad,
                                            format!(
                                                "{}  {}",
                                                l("shortcut_conflict"),
                                                l(conflict.label())
                                            ),
                                        );
                                    }
                                });
                            }
                            ui.add_space(10.0);
                            egui::Grid::new("shortcut-grid")
                                .num_columns(3)
                                .spacing(Vec2::new(18.0, 10.0))
                                .striped(true)
                                .show(ui, |ui| {
                                    for binding in
                                        self.session.preferences.shortcuts.bindings.clone()
                                    {
                                        ui.label(l(binding.action.label()));
                                        pill(ui, binding.display(), p.soft, p.text);
                                        if ui.small_button(l("change")).clicked() {
                                            self.studio.capture = Some(binding.action);
                                            self.studio.conflict = None;
                                        }
                                        ui.end_row();
                                    }
                                });
                        }
                        _ => {
                            ui.label(
                                RichText::new(format!("Popy {}", env!("CARGO_PKG_VERSION")))
                                    .strong()
                                    .size(20.0),
                            );
                            let mut preview = self.session.preferences.preview_updates;
                            if ui.checkbox(&mut preview, l("include_previews")).changed() {
                                self.session.preferences.preview_updates = preview;
                                self.save_session();
                            }
                            ui.add_space(12.0);
                            let status = self.studio.update.status;
                            let busy = matches!(status, "checking" | "downloading");
                            if self.action_button(ui, Action::Updates, false, !busy, p) {
                                self.check_updates();
                            }
                            if !status.is_empty() {
                                ui.add_space(10.0);
                                ui.label(RichText::new(l(status)).color(
                                    if status == "update_error" {
                                        p.bad
                                    } else {
                                        p.text
                                    },
                                ));
                            }
                            if let Some(manifest) = &self.studio.update.manifest {
                                ui.label(
                                    RichText::new(format!(
                                        "v{} · {:.1} MB",
                                        manifest.version,
                                        manifest.archive_size as f64 / 1_000_000.0
                                    ))
                                    .color(p.muted),
                                );
                            }
                            if status == "update_available" {
                                if ui
                                    .add_enabled(
                                        self.active.is_none() && !self.queue_enabled,
                                        egui::Button::new(
                                            RichText::new(l("download_install"))
                                                .color(Color32::WHITE),
                                        )
                                        .fill(p.accent),
                                    )
                                    .clicked()
                                {
                                    self.download_update();
                                }
                                if self.active.is_some()
                                    || self.queue_enabled
                                    || self.studio.setup.is_some()
                                {
                                    ui.label(RichText::new(l("update_busy_hint")).color(p.muted));
                                }
                            }
                            if status == "checking" {
                                ui.spinner();
                            }
                            if status == "downloading" {
                                ui.add(
                                    egui::ProgressBar::new(self.studio.update.fraction)
                                        .fill(p.accent)
                                        .show_percentage(),
                                );
                                if ui.button(l("cancel_download")).clicked() {
                                    self.studio.update.cancel.store(true, Ordering::Relaxed);
                                    self.studio.update.auto_install = false;
                                }
                            }
                            if status == "update_ready" && ui.button(l("install_restart")).clicked()
                            {
                                self.studio.update.auto_install = true;
                            }
                            if !self.studio.update.error.is_empty() {
                                egui::CollapsingHeader::new(l("show_details")).show(ui, |ui| {
                                    ui.small(&self.studio.update.error);
                                });
                            }
                        }
                    });
            });
        self.studio.settings_open = open;
        if !open {
            self.studio.capture = None;
        }
    }

    fn resize_handles(&self, ui: &mut egui::Ui, rect: Rect) {
        if self.studio.fullscreen || ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false)) {
            return;
        }
        use egui::ResizeDirection::*;
        let border = 5.0;
        let corner = 11.0;
        let areas = [
            (
                Rect::from_min_max(rect.min, Pos2::new(rect.left() + border, rect.bottom())),
                West,
                egui::CursorIcon::ResizeHorizontal,
            ),
            (
                Rect::from_min_max(Pos2::new(rect.right() - border, rect.top()), rect.max),
                East,
                egui::CursorIcon::ResizeHorizontal,
            ),
            (
                Rect::from_min_max(rect.min, Pos2::new(rect.right(), rect.top() + border)),
                North,
                egui::CursorIcon::ResizeVertical,
            ),
            (
                Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - border), rect.max),
                South,
                egui::CursorIcon::ResizeVertical,
            ),
            (
                Rect::from_min_size(rect.min, Vec2::splat(corner)),
                NorthWest,
                egui::CursorIcon::ResizeNwSe,
            ),
            (
                Rect::from_min_size(
                    Pos2::new(rect.right() - corner, rect.top()),
                    Vec2::splat(corner),
                ),
                NorthEast,
                egui::CursorIcon::ResizeNeSw,
            ),
            (
                Rect::from_min_size(
                    Pos2::new(rect.left(), rect.bottom() - corner),
                    Vec2::splat(corner),
                ),
                SouthWest,
                egui::CursorIcon::ResizeNeSw,
            ),
            (
                Rect::from_min_size(rect.max - Vec2::splat(corner), Vec2::splat(corner)),
                SouthEast,
                egui::CursorIcon::ResizeNwSe,
            ),
        ];
        for (index, (rect, direction, cursor)) in areas.into_iter().enumerate() {
            if ui
                .interact(
                    rect,
                    egui::Id::unique(("resize-edge", index)),
                    Sense::drag(),
                )
                .on_hover_cursor(cursor)
                .drag_started()
            {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
            }
        }
    }
    fn smoke_tick(&mut self, ctx: &egui::Context) {
        let settled = self
            .studio
            .completed
            .is_none_or(|time| time.elapsed() > Duration::from_millis(700));
        let Some(smoke) = &mut self.smoke else {
            return;
        };
        smoke.frames += 1;
        let keyboard_done = !smoke.keyboard_test
            || (self.studio.keyboard_step >= 11
                && self.studio.started.elapsed() > Duration::from_millis(1200));
        let setup_done =
            !smoke.setup_test || matches!(self.studio.setup_state, "completed" | "failed");
        let finished = keyboard_done
            && setup_done
            && (!smoke.run_export
                || (!self.queue_enabled
                    && self.active.is_none()
                    && !self.session.entries.is_empty()));
        if finished && settled && smoke.frames > 20 && !smoke.requested {
            smoke.requested = true;
            let path = smoke.screenshot.clone();
            let result = Arc::clone(&smoke.result);
            let keyboard_ok = !smoke.keyboard_test
                || (self.session.preferences.dark
                    && self.session.preferences.companion == 1
                    && self.session.settings.auto_color
                    && self.session.settings.model == Model::Anime
                    && self.split == 0.0
                    && self.studio.zoom == 1.0
                    && !self.studio.settings_open);
            let setup_ok =
                !smoke.setup_test || (self.runtime_ready && self.studio.setup_state == "completed");
            let success = keyboard_ok
                && setup_ok
                && (!smoke.run_export
                    || self
                        .session
                        .entries
                        .iter()
                        .all(|entry| entry.state == "completed"));
            let _ = popy::pipeline::write_json(
                &path.with_extension("json"),
                &serde_json::json!({"setup_test":smoke.setup_test,"setup_passed":setup_ok,"keyboard_test":smoke.keyboard_test,"keyboard_passed":keyboard_ok,
                "locale":self.session.preferences.locale,"dark":self.session.preferences.dark,"companion":self.session.preferences.companion,
                "color":self.session.settings.auto_color,"model":self.session.settings.model,"success":success}),
            );
            ctx.request_screenshot(move |image| {
                let saved = image::save_buffer(
                    path,
                    image.as_raw(),
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                )
                .is_ok();
                result.store(if success && saved { 1 } else { 2 }, Ordering::SeqCst);
            });
        }
        if smoke.result.load(Ordering::SeqCst) != 0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl eframe::App for PopyApp {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        if self.smoke.as_ref().is_none_or(|s| !s.keyboard_test)
            || self.studio.started.elapsed() < Duration::from_millis(500)
        {
            return;
        }
        use egui::{Key, Modifiers};
        let ctrl = Modifiers {
            ctrl: true,
            command: true,
            ..Modifiers::NONE
        };
        let alt = Modifiers {
            alt: true,
            ..Modifiers::NONE
        };
        let keys = [
            (Key::F6, Modifiers::NONE),
            (Key::B, ctrl),
            (Key::C, alt),
            (Key::Num2, alt),
            (Key::Comma, ctrl),
            (Key::Escape, Modifiers::NONE),
            (Key::B, Modifiers::NONE),
            (Key::A, Modifiers::NONE),
            (Key::Num0, ctrl),
            (Key::F1, Modifiers::NONE),
            (Key::Escape, Modifiers::NONE),
        ];
        if let Some(&(key, modifiers)) = keys.get(self.studio.keyboard_step) {
            for pressed in [true, false] {
                input.events.push(egui::Event::Key {
                    key,
                    physical_key: Some(key),
                    pressed,
                    repeat: false,
                    modifiers,
                });
            }
            self.studio.keyboard_step += 1;
        }
    }
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let outer = ui.max_rect();
        self.poll_setup();
        let was_running = self.active.is_some();
        self.poll(&ctx);
        if was_running
            && self.active.is_none()
            && self
                .selected
                .and_then(|i| self.session.entries.get(i))
                .is_some_and(|entry| entry.state == "completed")
        {
            self.studio.completed = Some(Instant::now());
        }
        self.poll_updates(&ctx);
        if self.studio.update_test.is_some() && self.studio.update.status.is_empty() {
            self.check_updates();
        }
        if !self.runtime_ready
            && self.studio.setup.is_none()
            && self.studio.runtime_check.elapsed() > Duration::from_secs(2)
        {
            self.studio.runtime_check = Instant::now();
            if Tools::load(&self.root).is_ok() {
                self.runtime_ready = true;
                self.message.clear();
            }
        }
        let p = configure(&ctx, &self.session.preferences);
        if self.studio.settings_open
            && self.studio.capture.is_none()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.studio.settings_open = false;
        } else {
            self.keyboard(&ctx);
        }
        let dropped = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_owned())
                .collect::<Vec<_>>()
        });
        if !dropped.is_empty() {
            self.add_files(dropped);
        }
        if ctx.input(|i| i.viewport().close_requested())
            && (self.active.is_some() || self.studio.setup.is_some())
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_close = true;
        }
        let foreground = ctx.input(|i| {
            i.viewport().focused.unwrap_or(true) && !i.viewport().minimized.unwrap_or(false)
        });
        ctx.request_repaint_after(Duration::from_millis(
            if foreground
                && !self.session.preferences.reduced_motion
                && self.session.preferences.companion != 2
            {
                16
            } else {
                200
            },
        ));
        egui::Frame::new()
            .fill(p.bg)
            .corner_radius(20.0)
            .stroke(Stroke::new(1.0, p.line))
            .inner_margin(1.0)
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                self.title_bar(ui, p);
                egui::Panel::left("library-panel")
                    .resizable(true)
                    .default_size(236.0)
                    .min_size(195.0)
                    .max_size(320.0)
                    .frame(egui::Frame::new().fill(p.bg).inner_margin(16.0))
                    .show(ui, |ui| {
                        self.library(ui, p);
                    });
                egui::Panel::right("inspector-panel")
                    .resizable(true)
                    .default_size(288.0)
                    .min_size(255.0)
                    .max_size(360.0)
                    .frame(egui::Frame::new().fill(p.bg).inner_margin(16.0))
                    .show(ui, |ui| {
                        self.inspector(ui, p);
                    });
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(p.bg).inner_margin(egui::Margin {
                        left: 12,
                        right: 12,
                        top: 16,
                        bottom: 16,
                    }))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("workspace-scroll")
                            .show(ui, |ui| {
                                if !self.runtime_ready || self.studio.setup.is_some() {
                                    card(p).fill(p.tint).show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(self.l(if self.studio.setup.is_some() {
                                                "downloading"
                                            } else {
                                                "setup_needed"
                                            }));
                                            if self.studio.setup.is_some() {
                                                ui.spinner();
                                            } else if ui.button(self.l("setup_tools")).clicked() {
                                                self.perform(Action::Setup, ui.ctx());
                                            }
                                            if self.studio.setup.is_some()
                                                && ui.button(self.l("stop")).clicked()
                                            {
                                                self.perform(Action::Stop, ui.ctx());
                                            }
                                        });
                                        if !self.message.is_empty() {
                                            egui::CollapsingHeader::new(self.l("show_details"))
                                                .show(ui, |ui| {
                                                    ui.small(&self.message);
                                                });
                                        }
                                    });
                                    ui.add_space(10.0);
                                }
                                self.canvas(ui, p);
                            });
                    });
            });
        self.resize_handles(ui, outer);
        self.preferences_window(&ctx, p);
        if self.confirm_close {
            egui::Window::new(self.l(if self.active.is_some() {
                "close_title"
            } else {
                "setup_tools"
            }))
            .id(egui::Id::unique("confirm-close"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(&ctx, |ui| {
                ui.label(self.l("close_hint"));
                ui.add_space(9.0);
                ui.horizontal(|ui| {
                    if ui.button(self.l("continue_work")).clicked() {
                        self.confirm_close = false;
                    }
                    if ui.button(self.l("stop_exit")).clicked() {
                        self.cancel();
                        self.active.take();
                        self.studio.setup.take();
                        self.confirm_close = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        }
        self.smoke_tick(&ctx);
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.studio.update.cancel.store(true, Ordering::Relaxed);
        self.cancel();
        self.active.take();
        self.studio.setup.take();
        self.save_session();
    }
}
