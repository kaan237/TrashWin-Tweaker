#![windows_subsystem = "windows"]

mod apps;
mod engine;
mod tweaks;

use eframe::egui::{self, Color32, RichText};
use engine::Status;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, Receiver, Sender};
use tweaks::{Cat, Risk, Tweak, TWEAKS};

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Tweaks(Cat),
    Apps,
    Log,
}

enum Job {
    Apply(Vec<&'static Tweak>),
    Revert(Vec<&'static Tweak>),
    Install(Vec<&'static apps::Pkg>),
}

enum Msg {
    Line(String),
    Done,
}

struct Gui {
    page: Page,
    picked: HashSet<&'static str>,
    picked_apps: HashSet<&'static str>,
    status: HashMap<&'static str, Status>,
    log: Vec<String>,
    filter: String,
    restore_first: bool,
    busy: bool,
    confirm: Option<Job>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    file_log: engine::Logger,
}

const ACCENT: Color32 = Color32::from_rgb(0x4a, 0xde, 0x80);
const WARN: Color32 = Color32::from_rgb(0xfb, 0xbf, 0x24);
const DANGER: Color32 = Color32::from_rgb(0xf8, 0x71, 0x71);
const MUTED: Color32 = Color32::from_rgb(0x9c, 0xa3, 0xaf);

fn main() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TrashWin")
            .with_inner_size([1040.0, 700.0])
            .with_min_inner_size([780.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native("TrashWin", opts, Box::new(|cc| Ok(Box::new(Gui::new(cc)))))
}

impl Gui {
    fn new(cc: &eframe::CreationContext) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let (tx, rx) = channel();
        let mut g = Gui {
            page: Page::Tweaks(Cat::Privacy),
            picked: HashSet::new(),
            picked_apps: HashSet::new(),
            status: HashMap::new(),
            log: Vec::new(),
            filter: String::new(),
            restore_first: true,
            busy: false,
            confirm: None,
            tx,
            rx,
            file_log: engine::Logger::open(),
        };
        g.push(format!("TrashWin {} başladı. Yedek klasörü: {}", env!("CARGO_PKG_VERSION"), engine::data_dir().display()));
        g.refresh();
        g
    }

    fn push(&mut self, s: String) {
        self.file_log.line(&s);
        self.log.push(s);
    }

    fn refresh(&mut self) {
        for t in TWEAKS {
            self.status.insert(t.id, engine::status(t));
        }
    }

    /// Riskli ayar varsa önce onay penceresi, yoksa doğrudan çalıştır.
    fn request(&mut self, job: Job) {
        let risky = matches!(&job, Job::Apply(ts) if ts.iter().any(|t| t.risk == Risk::Risky));
        if risky {
            self.confirm = Some(job);
        } else {
            self.start(job);
        }
    }

    fn start(&mut self, job: Job) {
        self.busy = true;
        self.page = Page::Log;
        let tx = self.tx.clone();
        let restore = self.restore_first;
        std::thread::spawn(move || {
            let send = |s: String| {
                let _ = tx.send(Msg::Line(s));
            };
            let mut log = |s: String| send(s);
            match job {
                Job::Apply(ts) => {
                    if restore && ts.iter().any(|t| t.undoable()) {
                        send("▶ Geri yükleme noktası oluşturuluyor...".into());
                        match engine::run(&tweaks::Cmd::Ps(tweaks::RESTORE_POINT)) {
                            Ok(_) => send("  ✔ Geri yükleme noktası hazır".into()),
                            Err(e) => send(format!("  ! Geri yükleme noktası oluşmadı (devam ediliyor): {e}")),
                        }
                    }
                    run_all(&ts, "Uygulanıyor", engine::apply, &mut log);
                }
                Job::Revert(ts) => run_all(&ts, "Geri alınıyor", engine::revert, &mut log),
                Job::Install(ps) => {
                    if !engine::winget_available() {
                        send("✖ WinGet bulunamadı. Microsoft Store'dan 'Uygulama Yükleyici'yi (App Installer) kur.".into());
                    } else {
                        let mut ok = 0;
                        for (i, p) in ps.iter().enumerate() {
                            send(format!("▶ [{}/{}] {} kuruluyor...", i + 1, ps.len(), p.name));
                            match engine::install(p) {
                                Ok(_) => {
                                    ok += 1;
                                    send(format!("  ✔ {} hazır", p.name));
                                }
                                Err(e) => send(format!("  ✖ {e}")),
                            }
                        }
                        send(format!("» Kurulum bitti: {ok}/{} başarılı", ps.len()));
                    }
                }
            }
            let _ = tx.send(Msg::Done);
        });
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.heading(RichText::new("TrashWin").color(ACCENT).strong());
        ui.label(RichText::new("geri alınabilir Windows ayarları").small().color(MUTED));
        ui.add_space(14.0);
        for c in Cat::ALL {
            let n = TWEAKS.iter().filter(|t| t.cat == c && self.status.get(t.id) == Some(&Status::Applied)).count();
            let total = TWEAKS.iter().filter(|t| t.cat == c).count();
            let txt = format!("{}   {n}/{total}", c.label());
            if ui.selectable_label(self.page == Page::Tweaks(c), txt).clicked() {
                self.page = Page::Tweaks(c);
            }
        }
        ui.separator();
        if ui.selectable_label(self.page == Page::Apps, "Uygulama Kur").clicked() {
            self.page = Page::Apps;
        }
        if ui.selectable_label(self.page == Page::Log, "Günlük").clicked() {
            self.page = Page::Log;
        }
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.add_space(8.0);
            ui.hyperlink_to("GitHub", "https://github.com/kaan237/TrashWin");
            ui.label(RichText::new(format!("v{}", env!("CARGO_PKG_VERSION"))).small().color(MUTED));
        });
    }

    fn tweak_page(&mut self, ui: &mut egui::Ui, cat: Cat) {
        let q = self.filter.to_lowercase();
        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            for t in TWEAKS.iter().filter(|t| t.cat == cat) {
                if !q.is_empty() && !t.name.to_lowercase().contains(&q) && !t.desc.to_lowercase().contains(&q) {
                    continue;
                }
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let mut on = self.picked.contains(t.id);
                        if ui.checkbox(&mut on, RichText::new(t.name).strong().size(15.0)).changed() {
                            toggle(&mut self.picked, t.id, on);
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (txt, col) = match self.status.get(t.id).copied().unwrap_or(Status::Unknown) {
                                Status::Applied => ("● uygulandı", ACCENT),
                                Status::Partial => ("● kısmen", WARN),
                                Status::NotApplied => ("○ varsayılan", MUTED),
                                Status::Unknown if t.undoable() => ("geri alınabilir", MUTED),
                                Status::Unknown => ("tek seferlik", MUTED),
                            };
                            ui.label(RichText::new(txt).color(col));
                            let (r, rc) = match t.risk {
                                Risk::Safe => ("güvenli", ACCENT),
                                Risk::Caution => ("dikkat", WARN),
                                Risk::Risky => ("riskli", DANGER),
                            };
                            ui.label(RichText::new(r).color(rc).small());
                            if t.reboot {
                                ui.label(RichText::new("yeniden başlatma").color(MUTED).small());
                            }
                        });
                    });
                    ui.label(RichText::new(t.desc).color(MUTED));
                    ui.push_id(t.id, |ui| {
                        ui.collapsing("Ne değişecek?", |ui| {
                            for r in t.regs {
                                ui.monospace(r.to_string());
                            }
                            for c in t.apply {
                                ui.monospace(format!("çalıştır: {c}"));
                            }
                            for c in t.undo {
                                ui.monospace(format!("geri alırken: {c}"));
                            }
                        });
                    });
                });
                ui.add_space(4.0);
            }
        });
    }

    fn apps_page(&mut self, ui: &mut egui::Ui) {
        let q = self.filter.to_lowercase();
        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            for g in apps::groups() {
                let items: Vec<_> = apps::PKGS
                    .iter()
                    .filter(|p| p.group == g && (q.is_empty() || p.name.to_lowercase().contains(&q)))
                    .collect();
                if items.is_empty() {
                    continue;
                }
                ui.add_space(6.0);
                ui.label(RichText::new(g).strong().color(ACCENT));
                ui.horizontal_wrapped(|ui| {
                    for p in items {
                        let mut on = self.picked_apps.contains(p.id);
                        if ui.checkbox(&mut on, p.name).on_hover_text(format!("{} ({})", p.id, p.source)).changed() {
                            toggle(&mut self.picked_apps, p.id, on);
                        }
                        ui.add_space(10.0);
                    }
                });
            }
        });
    }

    fn log_page(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().auto_shrink([false; 2]).stick_to_bottom(true).show(ui, |ui| {
            for l in &self.log {
                let col = if l.contains('✖') {
                    DANGER
                } else if l.contains('!') {
                    WARN
                } else {
                    Color32::LIGHT_GRAY
                };
                ui.label(RichText::new(l).monospace().color(col));
            }
        });
    }

    fn bottom_bar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let idle = !self.busy;
            if self.page == Page::Apps {
                if ui.add_enabled(idle, egui::Button::new("Temel paketi seç")).clicked() {
                    self.picked_apps.extend(apps::ESSENTIALS.iter().copied());
                }
                if ui.add_enabled(idle, egui::Button::new("Seçimi temizle")).clicked() {
                    self.picked_apps.clear();
                }
                let n = self.picked_apps.len();
                let btn = egui::Button::new(RichText::new(format!("{n} uygulamayı kur")).strong());
                if ui.add_enabled(idle && n > 0, btn).clicked() {
                    let ps = apps::PKGS.iter().filter(|p| self.picked_apps.contains(p.id)).collect();
                    self.request(Job::Install(ps));
                }
            } else {
                if ui.add_enabled(idle, egui::Button::new("Önerilenleri seç")).on_hover_text("Tüm 'güvenli' ve geri alınabilir ayarlar").clicked() {
                    self.picked.extend(TWEAKS.iter().filter(|t| t.risk == Risk::Safe && t.undoable()).map(|t| t.id));
                }
                if ui.add_enabled(idle, egui::Button::new("Seçimi temizle")).clicked() {
                    self.picked.clear();
                }
                ui.checkbox(&mut self.restore_first, "Önce geri yükleme noktası");
                let sel: Vec<&'static Tweak> = TWEAKS.iter().filter(|t| self.picked.contains(t.id)).collect();
                let undoable: Vec<_> = sel.iter().copied().filter(|t| t.undoable()).collect();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.busy {
                        ui.spinner();
                    }
                    let apply = egui::Button::new(RichText::new(format!("{} ayarı uygula", sel.len())).strong().color(Color32::BLACK)).fill(ACCENT);
                    if ui.add_enabled(idle && !sel.is_empty(), apply).clicked() {
                        self.request(Job::Apply(sel.clone()));
                    }
                    if ui.add_enabled(idle && !undoable.is_empty(), egui::Button::new(format!("{} ayarı geri al", undoable.len()))).clicked() {
                        self.request(Job::Revert(undoable));
                    }
                });
            }
        });
        ui.add_space(6.0);
    }

    fn confirm_window(&mut self, ctx: &egui::Context) {
        let Some(Job::Apply(ts)) = &self.confirm else { return };
        let risky: Vec<&Tweak> = ts.iter().copied().filter(|t| t.risk == Risk::Risky).collect();
        let mut answer = None;
        egui::Window::new("Riskli ayar onayı")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_max_width(460.0);
                ui.label("Aşağıdaki ayarlar güvenliği veya bazı özellikleri etkiler:");
                for t in &risky {
                    ui.add_space(4.0);
                    ui.label(RichText::new(t.name).strong().color(DANGER));
                    ui.label(t.desc);
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Vazgeç").clicked() {
                        answer = Some(false);
                    }
                    if ui.button(RichText::new("Anladım, uygula").color(DANGER)).clicked() {
                        answer = Some(true);
                    }
                });
            });
        match answer {
            Some(true) => {
                if let Some(job) = self.confirm.take() {
                    self.start(job);
                }
            }
            Some(false) => self.confirm = None,
            None => {}
        }
    }
}

fn toggle(set: &mut HashSet<&'static str>, id: &'static str, on: bool) {
    if on {
        set.insert(id);
    } else {
        set.remove(id);
    }
}

type Step = fn(&Tweak, &mut dyn FnMut(String)) -> Result<(), String>;

fn run_all(
    ts: &[&'static Tweak],
    verb: &str,
    f: Step,
    log: &mut dyn FnMut(String),
) {
    let mut ok = 0;
    for t in ts {
        log(format!("▶ {verb}: {}", t.name));
        match f(t, log) {
            Ok(()) => ok += 1,
            Err(e) => log(format!("  ✖ {e}")),
        }
    }
    let reboot = ts.iter().any(|t| t.reboot);
    log(format!(
        "» {ok}/{} tamamlandı{}",
        ts.len(),
        if reboot { ". Bazı değişiklikler için yeniden başlat." } else { "" }
    ));
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Line(s) => self.push(s),
                Msg::Done => {
                    self.busy = false;
                    self.refresh();
                }
            }
        }
        if self.busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
        }

        egui::SidePanel::left("nav").resizable(false).exact_width(190.0).show(ctx, |ui| self.sidebar(ui));
        egui::TopBottomPanel::bottom("bar").show(ctx, |ui| self.bottom_bar(ui));
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.page != Page::Log {
                ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Ara...").desired_width(f32::INFINITY));
                ui.add_space(6.0);
            }
            match self.page {
                Page::Tweaks(c) => self.tweak_page(ui, c),
                Page::Apps => self.apps_page(ui),
                Page::Log => self.log_page(ui),
            }
        });
        self.confirm_window(ctx);
    }
}
