#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use eframe::egui::{self, Color32, RichText, TextureHandle, Vec2};
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

const ORANGE: Color32 = Color32::from_rgb(255, 156, 79);
const MUTED: Color32 = Color32::from_rgb(157, 164, 177);
const BACKGROUND: Color32 = Color32::from_rgb(22, 25, 31);
const PANEL: Color32 = Color32::from_rgb(30, 34, 42);
const LOGO: &[u8] = include_bytes!("../branding/popy-logo-v1.png");

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
    state: String,
    progress: Option<Progress>,
    error: String,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Session {
    settings: Settings,
    entries: Vec<Entry>,
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
}
struct PopyApp {
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
        let mut style = (*cc.egui_ctx.style_of(egui::Theme::Dark)).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = BACKGROUND;
        style.visuals.window_fill = PANEL;
        style.visuals.selection.bg_fill = ORANGE.gamma_multiply(0.28);
        style.visuals.selection.stroke.color = ORANGE;
        style.visuals.widgets.active.bg_fill = ORANGE.gamma_multiply(0.45);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(65, 55, 47);
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.button_padding = Vec2::new(14.0, 10.0);
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        cc.egui_ctx.set_style_of(egui::Theme::Dark, style);
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
            command.args([
                "--preview-seconds",
                if self.smoke.is_some() { "1" } else { "3" },
            ]);
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
                Ok(()) => self.message = "İptal ediliyor; günlükler korunacak.".into(),
                Err(e) => self.message = format!("İptal dosyası yazılamadı: {e}"),
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
            .map(|i| self.session.entries[i].output.join("comparison.png"))
            .filter(|p| p.is_file());
        if image_path != self.comparison_path {
            self.comparison = image_path
                .as_ref()
                .and_then(|path| fs::read(path).ok())
                .and_then(|b| texture(ctx, &b, "comparison"));
            self.comparison_path = image_path;
        }
    }
    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            if let Some(logo) = &self.logo {
                ui.image((logo.id(), Vec2::splat(62.0)));
            }
            ui.vertical(|ui| {
                ui.label(RichText::new("popy").size(32.0).strong());
                ui.label(
                    RichText::new("Videona biraz hayat kat.")
                        .color(MUTED)
                        .size(12.0),
                );
            });
        });
        ui.add_space(28.0);
        ui.label(
            RichText::new("İYİLEŞTİRME")
                .color(MUTED)
                .size(11.0)
                .strong(),
        );
        ui.add_space(4.0);
        ui.label("İçerik türü");
        egui::ComboBox::from_id_salt("model")
            .width(226.0)
            .selected_text(if self.session.settings.model == Model::General {
                "Gerçek çekim · Genel"
            } else {
                "Anime / çizim · Hızlı"
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.session.settings.model,
                    Model::General,
                    "Gerçek çekim · Genel",
                );
                ui.selectable_value(
                    &mut self.session.settings.model,
                    Model::Anime,
                    "Anime / çizim · Hızlı",
                );
            });
        ui.label(
            RichText::new(if self.session.settings.model == Model::General {
                "Genel model daha ağırdır. iGPU'da önce 3 saniyelik önizleme dene."
            } else {
                "Anime modeli gerçek çekimde ayrıntıları değiştirebilir."
            })
            .color(MUTED)
            .size(12.0),
        );
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("2×").color(ORANGE).size(24.0).strong());
            ui.label("Çözünürlük artışı");
        });
        ui.add_space(12.0);
        ui.checkbox(&mut self.session.settings.auto_color, "Otomatik renk");
        ui.label(
            RichText::new("Ton, kontrast ve hafif doygunluk. Siyah beyaza yeni renk eklemez.")
                .color(MUTED)
                .size(12.0),
        );
        ui.add_space(16.0);
        ui.collapsing("Ekran kartı ve bellek", |ui| {
            ui.horizontal(|ui| { ui.label("GPU indeksi"); ui.add(egui::DragValue::new(&mut self.session.settings.gpu).range(0..=15)); });
            ui.horizontal(|ui| { ui.label("Tile"); ui.add(egui::DragValue::new(&mut self.session.settings.tile).range(32..=256).speed(8)); });
            ui.label(RichText::new("Vulkan sürücüsü gerekir. Varsayılan GPU 0. Bellek hatasında tile değerini düşür. Cihaz listesi işin ai günlüğünde.").size(11.0).color(MUTED));
        });
        ui.add_space(14.0);
        ui.separator();
        ui.add_space(8.0);
        ui.label("Kayıt klasörü");
        let folder = &self.session.settings.output;
        let short = format!(
            "… / {} / {}",
            folder.parent().map(filename).unwrap_or_default(),
            filename(folder)
        );
        ui.label(RichText::new(short).size(11.0).color(MUTED))
            .on_hover_text(folder.display().to_string());
        if ui.button("Klasörü değiştir").clicked()
            && let Some(path) = rfd::FileDialog::new()
                .set_title("Popy çıktı klasörü")
                .pick_folder()
        {
            self.session.settings.output = path;
            self.save_session();
        }
        ui.add_space(20.0);
        ui.label(
            RichText::new("Her şey bilgisayarında işlenir.")
                .size(12.0)
                .color(MUTED),
        );
        ui.label(
            RichText::new(format!("v{} · Ön sürüm", env!("CARGO_PKG_VERSION")))
                .color(ORANGE)
                .size(11.0),
        );
        ui.hyperlink_to("Kaynak kodu", "https://github.com/boranseasonnew/Popy");
    }
    fn preview(&mut self, ui: &mut egui::Ui) {
        let available = Vec2::new(ui.available_width(), 300.0);
        egui::Frame::new()
            .fill(PANEL)
            .corner_radius(12.0)
            .inner_margin(18.0)
            .show(ui, |ui| {
                ui.set_min_size(available - Vec2::splat(36.0));
                if let Some(texture) = &self.comparison {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Kaynak").color(MUTED));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("Popy · 2×").color(ORANGE));
                        });
                    });
                    let [width, height] = texture.size();
                    let ratio = (width as f32 / 2.0) / height as f32;
                    let w = ui.available_width().min(236.0 * ratio);
                    let rect = ui
                        .vertical_centered(|ui| {
                            ui.allocate_exact_size(Vec2::new(w, w / ratio), egui::Sense::hover())
                                .0
                        })
                        .inner;
                    ui.painter().image(
                        texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.5, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    let x = rect.left() + rect.width() * self.split;
                    let left = egui::Rect::from_min_max(rect.min, egui::pos2(x, rect.bottom()));
                    ui.painter().with_clip_rect(left).image(
                        texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(0.5, 1.0)),
                        Color32::WHITE,
                    );
                    ui.painter().line_segment(
                        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                        egui::Stroke::new(2.0, ORANGE),
                    );
                    ui.vertical_centered(|ui| {
                        ui.spacing_mut().slider_width = (w - 110.0).max(100.0);
                        ui.add(
                            egui::Slider::new(&mut self.split, 0.0..=1.0)
                                .show_value(false)
                                .text("Önce / sonra"),
                        );
                    });
                    ui.label(
                        RichText::new(
                            "İlk kare karşılaştırması · Hareketli sonucu videoyu açarak incele.",
                        )
                        .size(11.0)
                        .color(MUTED),
                    );
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(38.0);
                        if let Some(logo) = &self.logo {
                            ui.image((logo.id(), Vec2::splat(92.0)));
                        }
                        ui.label(
                            RichText::new("Küçük bir dokunuş, daha net bir video.").size(18.0),
                        );
                        ui.label(
                            RichText::new("Video ekle, ayarını seç ve kısa bir önizleme oluştur.")
                                .color(MUTED),
                        );
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("Videoları bu pencereye de sürükleyebilirsin.")
                                .color(MUTED)
                                .size(12.0),
                        );
                    });
                }
            });
    }
    fn queue(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("İş kuyruğu").size(17.0).strong());
            ui.label(RichText::new(format!("{} iş", self.session.entries.len())).color(MUTED));
        });
        egui::ScrollArea::vertical()
            .id_salt("queue")
            .max_height(155.0)
            .show(ui, |ui| {
                if self.session.entries.is_empty() {
                    ui.label(RichText::new("Henüz video eklenmedi.").color(MUTED));
                }
                for (i, entry) in self.session.entries.iter().enumerate() {
                    let status = match entry.state.as_str() {
                        "ready" => "Hazır",
                        "queued" => "Sırada",
                        "running" => "İşleniyor",
                        "completed" => "Tamamlandı",
                        "cancelled" => "İptal",
                        "interrupted" => "Yarıda kaldı",
                        _ => "Hata",
                    };
                    let text = format!(
                        "{}{}   ·   {}",
                        filename(&entry.input),
                        if entry.preview { "  [önizleme]" } else { "" },
                        status
                    );
                    if ui
                        .add(
                            egui::Button::new(text)
                                .selected(self.selected == Some(i))
                                .min_size(Vec2::new(ui.available_width(), 34.0)),
                        )
                        .clicked()
                    {
                        self.selected = Some(i);
                    }
                }
            });
    }
}

impl eframe::App for PopyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll(&ctx);
        ctx.request_repaint_after(Duration::from_millis(200));
        let dropped: Vec<_> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_owned())
                .collect()
        });
        if !dropped.is_empty() {
            self.add_files(dropped);
        }
        if ctx.input(|i| i.viewport().close_requested()) && self.active.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_close = true;
        }
        egui::Panel::left("settings")
            .resizable(false)
            .exact_size(280.0)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(20.0))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.sidebar(ui));
            });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(BACKGROUND).inner_margin(26.0)).show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("main").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| { ui.label(RichText::new("Biraz daha net.").size(27.0).strong()); ui.label(RichText::new("Yerel, açık kaynak video iyileştirme.").color(MUTED)); });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("+  Video ekle").clicked()
                        && let Some(paths) = rfd::FileDialog::new().set_title("Video seç").add_filter("Video", &["mp4","mkv","mov"]).pick_files() { self.add_files(paths); }
                });
            });
            ui.add_space(18.0);
            if !self.runtime_ready {
                ui.colored_label(ORANGE,"İşleme araçları hazır değil.");
                ui.label("Paketteki Araclari Kur.cmd dosyasını çalıştırıp Popy'yi yeniden aç.");
            }
            self.preview(ui);
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(self.runtime_ready && self.selected.is_some(), egui::Button::new("3 sn önizleme")).clicked() { self.schedule(true); }
                let ready = self.session.entries.iter().any(|e| e.state == "ready" || e.state == "queued");
                if ui.add_enabled(self.runtime_ready && ready, egui::Button::new(RichText::new("Kuyruğu işle").color(Color32::BLACK)).fill(ORANGE)).clicked() { self.schedule(false); }
                if ui.add_enabled(self.active.is_some() || self.queue_enabled, egui::Button::new("Durdur")).clicked() { self.cancel(); }
            });
            if let Some(active) = &self.active {
                let entry = &self.session.entries[active.index];
                let progress = entry.progress.as_ref();
                let fraction = progress.map(|p| if p.total_frames > 0 { p.completed_frames as f32 / p.total_frames as f32 * 0.92 } else { 0.0 }).unwrap_or(0.0);
                ui.add(egui::ProgressBar::new(fraction).fill(ORANGE).text(progress.map(|p| p.message.as_str()).unwrap_or("Başlatılıyor…")));
                ui.label(RichText::new(format!("{} · {:.0} sn", filename(&entry.input), active.start.elapsed().as_secs_f64())).color(MUTED).size(11.0));
            }
            if !self.message.is_empty() { ui.colored_label(ORANGE, &self.message); }
            ui.add_space(8.0);
            self.queue(ui);
            if let Some(selected) = self.selected {
                let entry = self.session.entries[selected].clone();
                if !entry.error.is_empty() { egui::ScrollArea::vertical().max_height(65.0).show(ui, |ui| { ui.colored_label(Color32::LIGHT_RED,&entry.error); }); }
                ui.horizontal(|ui| {
                    if entry.state == "completed" && ui.button("Videoyu aç").clicked() { open_path(&entry.output.join("enhanced.mp4")); }
                    if entry.output.is_dir() && ui.button("Sonuç klasörü").clicked() { open_path(&entry.output); }
                    if matches!(entry.state.as_str(),"failed"|"cancelled"|"interrupted") && ui.button("Tekrar dene").clicked() {
                        let mut retry = entry.clone(); retry.state = "queued".into(); retry.progress = None; retry.error.clear(); retry.output = PathBuf::new(); retry.settings = self.session.settings.clone();
                        self.session.entries.push(retry); self.selected = Some(self.session.entries.len()-1); self.queue_enabled = true; self.save_session();
                    }
                    if self.active.is_none() && ui.button("Tamamlananları listeden kaldır").clicked() {
                        self.session.entries.retain(|e| !matches!(e.state.as_str(),"completed"|"cancelled"|"failed"|"interrupted")); self.selected = None; self.save_session();
                    }
                });
            }
            ui.add_space(8.0);
            ui.label(RichText::new("Destek: H.264 / AAC · sabit FPS · BT.709 SDR. HDR, değişken FPS ve rotasyon henüz desteklenmiyor.").size(11.0).color(MUTED));
            });
        });
        if self.confirm_close {
            egui::Window::new("İş devam ediyor")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(&ctx, |ui| {
                    ui.label("Popy kapanırsa çalışan iş durdurulacak. Günlükler korunur.");
                    ui.horizontal(|ui| {
                        if ui.button("Çalışmaya devam et").clicked() {
                            self.confirm_close = false;
                        }
                        if ui.button("İşi durdur ve çık").clicked() {
                            self.cancel();
                            self.active.take();
                            self.confirm_close = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                });
        }
        if let Some(smoke) = &mut self.smoke {
            smoke.frames += 1;
            let finished = !smoke.run_export
                || (!self.queue_enabled
                    && self.active.is_none()
                    && !self.session.entries.is_empty());
            if finished && smoke.frames > 8 && !smoke.requested {
                smoke.requested = true;
                let path = smoke.screenshot.clone();
                let result = Arc::clone(&smoke.result);
                let success = !smoke.run_export
                    || self.session.entries.iter().all(|e| e.state == "completed");
                ctx.request_screenshot(move |image| {
                    let saved = image::save_buffer(
                        &path,
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
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.cancel();
        self.active.take();
        self.save_session();
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
        .with_inner_size([1220.0, 850.0])
        .with_min_inner_size([1000.0, 760.0]);
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    let output = value("--smoke-output");
    eframe::run_native(
        "Popy",
        eframe::NativeOptions {
            viewport,
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| Ok(Box::new(PopyApp::new(cc, root, smoke, smoke_input, output)))),
    )?;
    if is_smoke && result.load(Ordering::SeqCst) != 1 {
        std::process::exit(1);
    }
    Ok(())
}
