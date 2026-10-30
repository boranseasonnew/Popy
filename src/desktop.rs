#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use eframe::egui::{self, TextureHandle};
use popy::{
    export::Progress,
    process::ManagedChild,
    tools::{Model, Tools},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const LOGO: &[u8] = include_bytes!("../branding/popy-logo-v1.png");
#[path = "desktop_view.rs"]
mod view;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    locale: String,
    dark: bool,
    reduced_motion: bool,
    companion: u8,
    ui_scale: f32,
    preview_seconds: u32,
    preview_updates: bool,
    update_result_seen: String,
    shortcuts: popy::shortcuts::Shortcuts,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            locale: "en".into(),
            dark: false,
            reduced_motion: false,
            companion: 0,
            ui_scale: 1.0,
            preview_seconds: 3,
            preview_updates: true,
            update_result_seen: String::new(),
            shortcuts: Default::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    model: Model,
    auto_color: bool,
    gpu: u32,
    tile: u32,
    output: PathBuf,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            model: Model::General,
            auto_color: false,
            gpu: 0,
            tile: 64,
            output: PathBuf::from("exports"),
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    input: PathBuf,
    output: PathBuf,
    settings: Settings,
    preview: bool,
    #[serde(default = "default_preview_seconds")]
    preview_seconds: u32,
    state: String,
    progress: Option<Progress>,
    error: String,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Session {
    settings: Settings,
    entries: Vec<Entry>,
    preferences: Preferences,
}
fn default_preview_seconds() -> u32 {
    3
}
struct Running {
    managed: ManagedChild,
    index: usize,
    cancel: PathBuf,
    log: PathBuf,
    start: Instant,
}
struct Smoke {
    screenshot: PathBuf,
    requested: bool,
    result: Arc<AtomicU8>,
    frames: u32,
    run_export: bool,
    keyboard_test: bool,
    setup_test: bool,
}
struct PopyApp {
    studio: view::Studio,
    root: PathBuf,
    session: Session,
    selected: Option<usize>,
    active: Option<Running>,
    queue_enabled: bool,
    logo: Option<TextureHandle>,
    comparison: Option<TextureHandle>,
    comparison_path: Option<PathBuf>,
    split: f32,
    message: String,
    runtime_ready: bool,
    confirm_close: bool,
    smoke: Option<Smoke>,
}

fn unique() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
fn filename(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}
fn texture(ctx: &egui::Context, bytes: &[u8], name: &str) -> Option<TextureHandle> {
    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    Some(ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    ))
}

impl PopyApp {
    fn new(
        cc: &eframe::CreationContext<'_>,
        root: PathBuf,
        smoke: Option<Smoke>,
        smoke_input: Option<PathBuf>,
        smoke_output: Option<PathBuf>,
    ) -> Self {
        let mut session = if smoke.is_some() {
            Session::default()
        } else {
            fs::read(root.join(".tools/desktop-session.json"))
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default()
        };
        if session.settings.output.is_relative() {
            session.settings.output = root.join(&session.settings.output);
        }
        session.settings.gpu = session.settings.gpu.min(15);
        session.settings.tile = session.settings.tile.clamp(32, 256);
        session.preferences.ui_scale = session.preferences.ui_scale.clamp(0.85, 1.2);
        session.preferences.preview_seconds = session.preferences.preview_seconds.clamp(1, 10);
        session.preferences.companion = session.preferences.companion.min(2);
        session.preferences.shortcuts.fill_missing_defaults();
        cc.egui_ctx.set_zoom_factor(session.preferences.ui_scale);
        for entry in &mut session.entries {
            if entry.state == "running" || entry.state == "queued" {
                entry.state = "interrupted".into();
                entry.error = "Önceki oturum kapandı. Tekrar denemek için işi seç.".into();
            }
        }
        let ready = Tools::load(&root);
        let message = ready
            .as_ref()
            .err()
            .map(|e| format!("{e:#}"))
            .unwrap_or_default();
        let mut app = Self {
            studio: view::Studio::new(&cc.egui_ctx),
            root,
            session,
            selected: None,
            active: None,
            queue_enabled: false,
            logo: texture(&cc.egui_ctx, LOGO, "popy-logo"),
            comparison: None,
            comparison_path: None,
            split: 0.5,
            message,
            runtime_ready: ready.is_ok(),
            confirm_close: false,
            smoke,
        };
        if let Some(output) = smoke_output {
            app.session.settings.output = output;
        }
        if let Some(input) = smoke_input {
            app.session.settings.model = Model::Anime;
            app.session.settings.auto_color = true;
            app.add_files(vec![input]);
            if let Some(entry) = app.session.entries.last_mut() {
                entry.preview = true;
                entry.state = "queued".into();
            }
            app.queue_enabled = true;
        } else {
            app.selected = app.session.entries.len().checked_sub(1);
        }
        app.restore_update_result();
        app
    }
    fn save_session(&self) {
        if self.smoke.is_some() {
            return;
        }
        if let Ok(value) = serde_json::to_value(&self.session) {
            let _ = fs::create_dir_all(self.root.join(".tools"));
            let _ =
                popy::pipeline::write_json(&self.root.join(".tools/desktop-session.json"), &value);
        }
    }
    fn add_files(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            if !path.is_file() {
                continue;
            }
            self.session.entries.push(Entry {
                input: path,
                output: PathBuf::new(),
                settings: self.session.settings.clone(),
                preview: false,
                preview_seconds: self.session.preferences.preview_seconds,
                state: "ready".into(),
                progress: None,
                error: String::new(),
            });
        }
        self.selected = self.session.entries.len().checked_sub(1);
        self.save_session();
    }
    fn schedule(&mut self, preview: bool) {
        if preview {
            if let Some(selected) = self.selected {
                let mut entry = self.session.entries[selected].clone();
                entry.preview = true;
                entry.preview_seconds = self.session.preferences.preview_seconds;
                entry.settings = self.session.settings.clone();
                entry.state = "queued".into();
                entry.progress = None;
                entry.output = PathBuf::new();
                entry.error.clear();
                self.session.entries.push(entry);
                self.selected = Some(self.session.entries.len() - 1);
            }
        } else {
            for entry in &mut self.session.entries {
                if entry.state == "ready" {
                    entry.settings = self.session.settings.clone();
                    entry.state = "queued".into();
                }
            }
        }
        self.queue_enabled = true;
        self.save_session();
    }
    fn launch(&mut self, index: usize) -> anyhow::Result<()> {
        let entry = &mut self.session.entries[index];
        fs::create_dir_all(&entry.settings.output)?;
        let id = format!(
            "popy-{}{}",
            unique(),
            if entry.preview { "-preview" } else { "" }
        );
        entry.output = entry.settings.output.join(&id);
        let cancel = entry.settings.output.join(format!(".{id}.cancel"));
        let logs = self.root.join(".tools/gui-logs");
        fs::create_dir_all(&logs)?;
        let log = logs.join(format!("{id}.stderr.log"));
        let cli = std::env::current_exe()?.with_file_name(if cfg!(windows) {
            "popy.exe"
        } else {
            "popy"
        });
        anyhow::ensure!(
            cli.is_file(),
            "popy CLI bulunamadı. Paketin tüm dosyalarını aynı klasöre açın."
        );
        let mut command = Command::new(cli);
        command
            .current_dir(&self.root)
            .arg("enhance")
            .arg("--input")
            .arg(&entry.input)
            .arg("--out")
            .arg(&entry.output)
            .args([
                "--model",
                if entry.settings.model == Model::General {
                    "general"
                } else {
                    "anime"
                },
                "--gpu",
                &entry.settings.gpu.to_string(),
                "--tile",
                &entry.settings.tile.to_string(),
            ])
            .arg("--cancel-file")
            .arg(&cancel)
            .stdout(Stdio::null())
            .stderr(File::create(&log)?);
        if entry.settings.auto_color {
            command.arg("--auto-color");
        }
        if entry.preview {
            command.arg("--preview-seconds").arg(
                if self.smoke.is_some() {
                    1
                } else {
                    entry.preview_seconds
                }
                .to_string(),
            );
        }
        let managed = ManagedChild::spawn(&mut command)?;
        entry.state = "running".into();
        self.active = Some(Running {
            managed,
            index,
            cancel,
            log,
            start: Instant::now(),
        });
        self.save_session();
        Ok(())
    }
    fn cancel(&mut self) {
        self.queue_enabled = false;
        if let Some(active) = &self.active {
            match fs::write(&active.cancel, b"cancel") {
                Ok(()) => self.message = "Stopping the queue; diagnostic logs will be kept.".into(),
                Err(e) => self.message = format!("Could not request cancellation: {e}"),
            }
        }
    }
    fn poll(&mut self, ctx: &egui::Context) {
        let mut finished = None;
        if let Some(active) = &mut self.active {
            if let Ok(bytes) = fs::read(self.session.entries[active.index].output.join("job.json"))
                && let Ok(progress) = serde_json::from_slice::<Progress>(&bytes)
            {
                self.session.entries[active.index].progress = Some(progress);
            }
            match active.managed.child.try_wait() {
                Ok(Some(status)) => finished = Some(status.success()),
                Err(e) => {
                    self.message = format!("İş durumu okunamadı: {e}");
                    finished = Some(false);
                }
                _ => {}
            }
        }
        if let Some(success) = finished {
            let active = self.active.take().expect("active job");
            let entry = &mut self.session.entries[active.index];
            let state = entry
                .progress
                .as_ref()
                .map(|p| p.state.as_str())
                .unwrap_or("failed");
            entry.state = if success && state == "completed" {
                "completed"
            } else if state == "cancelled" {
                "cancelled"
            } else {
                "failed"
            }
            .into();
            if entry.state != "completed" {
                entry.error = entry
                    .progress
                    .as_ref()
                    .map(|p| p.message.clone())
                    .unwrap_or_else(|| {
                        fs::read_to_string(&active.log)
                            .unwrap_or_else(|_| "İş tamamlanamadı".into())
                    });
                if entry.error.len() > 8000 {
                    entry.error = entry.error.chars().take(4000).collect();
                }
            }
            let _ = fs::remove_file(&active.cancel);
            self.selected = Some(active.index);
            self.save_session();
        }
        if self.active.is_none() && self.queue_enabled && self.runtime_ready {
            if let Some(index) = self
                .session
                .entries
                .iter()
                .position(|e| e.state == "queued")
            {
                if let Err(e) = self.launch(index) {
                    self.session.entries[index].state = "failed".into();
                    self.session.entries[index].error = format!("{e:#}");
                    self.selected = Some(index);
                    self.save_session();
                }
            } else {
                self.queue_enabled = false;
            }
        }
        let image_path = self
            .selected
            .and_then(|i| {
                let entry = &self.session.entries[i];
                // FFmpeg can create the PNG before it has finished writing it.
                // Do not cache a failed texture decode from an in-flight job.
                (entry.state == "completed").then(|| entry.output.join("comparison.png"))
            })
            .filter(|p| p.is_file());
        if image_path != self.comparison_path {
            self.comparison = image_path
                .as_ref()
                .and_then(|path| fs::read(path).ok())
                .and_then(|b| texture(ctx, &b, "comparison"));
            self.comparison_path = image_path;
        }
    }
}

fn open_path(path: &Path) {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        // ShellExecute opens an existing file/folder with its registered app;
        // no shell command string or user-provided executable is composed.
        unsafe extern "system" {
            fn ShellExecuteW(
                window: *mut std::ffi::c_void,
                operation: *const u16,
                file: *const u16,
                params: *const u16,
                directory: *const u16,
                show: i32,
            ) -> isize;
        }
        let value: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        #[link(name = "shell32")]
        unsafe extern "system" {}
        unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                std::ptr::null(),
                value.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            );
        }
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new(if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        })
        .arg(path)
        .spawn();
    }
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    let value = |key: &str| {
        args.iter()
            .position(|a| a == key)
            .and_then(|i| args.get(i + 1))
            .map(PathBuf::from)
    };
    let result = Arc::new(AtomicU8::new(0));
    let smoke_input = value("--smoke-input");
    let smoke = value("--smoke-screenshot").map(|screenshot| Smoke {
        screenshot,
        requested: false,
        result: Arc::clone(&result),
        frames: 0,
        run_export: smoke_input.is_some(),
        keyboard_test: args.iter().any(|s| s == "--smoke-keys"),
        setup_test: args.iter().any(|s| s == "--smoke-setup"),
    });
    let is_smoke = smoke.is_some();
    let root = Tools::root().unwrap_or_else(|_| PathBuf::from("."));
    let icon = image::load_from_memory(LOGO).ok().map(|image| {
        let image = image
            .resize(128, 128, image::imageops::FilterType::Lanczos3)
            .to_rgba8();
        egui::IconData {
            width: image.width(),
            height: image.height(),
            rgba: image.into_raw(),
        }
    });
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1360.0, 900.0])
        .with_min_inner_size([1100.0, 760.0])
        .with_decorations(false)
        .with_transparent(true);
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    let output = value("--smoke-output");
    let locale = value("--smoke-locale").map(|p| p.to_string_lossy().into_owned());
    let dark = args.iter().any(|s| s == "--smoke-dark");
    let bear = args.iter().any(|s| s == "--smoke-bear");
    let settings = args.iter().any(|s| s == "--smoke-settings");
    let update_test = value("--smoke-update-result");
    let setup_test = args.iter().any(|s| s == "--smoke-setup");
    eframe::run_native(
        "Popy",
        eframe::NativeOptions {
            viewport,
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| {
            let mut app = PopyApp::new(cc, root, smoke, smoke_input, output);
            if let Some(locale) = locale {
                app.session.preferences.locale = locale;
            }
            if dark {
                app.session.preferences.dark = true;
            }
            if bear {
                app.session.preferences.companion = 1;
            }
            if settings {
                app.studio.settings_open = true;
            }
            app.studio.update_test = update_test;
            if setup_test {
                app.start_setup();
            }
            Ok(Box::new(app))
        }),
    )?;
    if is_smoke && result.load(Ordering::SeqCst) != 1 {
        std::process::exit(1);
    }
    Ok(())
}
