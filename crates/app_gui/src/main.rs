use eframe::{App, Frame, NativeOptions, egui};
use feeder_core::BgDiffDetector;
use feeder_core::{ImageInfo, ScanOptions, export_csv, scan_folder_detect};
use rfd::FileDialog;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::time::Instant;

fn main() {
    tracing_subscriber::fmt::init();
    let options = NativeOptions::default();
    if let Err(e) = eframe::run_native(
        "Feeder Vision (preview)",
        options,
        Box::new(|_cc| {
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(Box::new(UiApp::default()))
        }),
    ) {
        eprintln!("Applicatie gestopt met fout: {e}");
    }
}

#[derive(Default)]
struct UiApp {
    gekozen_map: Option<PathBuf>,
    rijen: Vec<ImageInfo>,
    bezig: bool,
    status: String,
    // Thumbnail cache (basic LRU)
    thumbs: HashMap<PathBuf, egui::TextureHandle>,
    thumb_keys: VecDeque<PathBuf>,
}

const THUMB_SIZE: u32 = 120;
const MAX_THUMBS: usize = 256;

impl UiApp {
    fn get_or_load_thumb(&mut self, ctx: &egui::Context, path: &Path) -> Option<egui::TextureId> {
        if let Some(tex) = self.thumbs.get(path) {
            return Some(tex.id());
        }

        match image::open(path) {
            Ok(img) => {
                let thumb = image::imageops::thumbnail(&img, THUMB_SIZE, THUMB_SIZE);
                let (w, h) = thumb.dimensions();
                let size = [w as usize, h as usize];
                let pixels = thumb.into_raw();
                let color = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                let name = format!("thumb:{}", path.display());
                let tex = ctx.load_texture(name, color, egui::TextureOptions::LINEAR);
                self.thumbs.insert(path.to_path_buf(), tex);
                self.thumb_keys.push_back(path.to_path_buf());
                if self.thumbs.len() > MAX_THUMBS
                    && let Some(old) = self.thumb_keys.pop_front()
                {
                    self.thumbs.remove(&old);
                }
                self.thumbs.get(path).map(|t| t.id())
            }
            Err(e) => {
                tracing::warn!("Failed to load thumbnail for {}: {}", path.display(), e);
                None
            }
        }
    }
}

impl App for UiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Kies map...").clicked()
                    && !self.bezig
                    && let Some(dir) = FileDialog::new().set_directory(".").pick_folder()
                {
                    self.gekozen_map = Some(dir);
                    self.rijen.clear();
                    self.status.clear();
                    self.thumbs.clear();
                    self.thumb_keys.clear();
                }

                let kan_scannen = self.gekozen_map.is_some() && !self.bezig;
                if ui
                    .add_enabled(kan_scannen, egui::Button::new("Scannen"))
                    .clicked()
                    && let Some(dir) = self.gekozen_map.clone()
                {
                    self.bezig = true;
                    self.status = "Bezig met scannen...".to_string();
                    // Blocking MVP scan; fine for v0
                    let start = Instant::now();
                    let mut detector = BgDiffDetector::default();
                    match scan_folder_detect(dir, ScanOptions { recursive: false }, &mut detector) {
                        Ok(rows) => {
                            let dur = start.elapsed();
                            let totaal = rows.len();
                            let aanwezig = rows.iter().filter(|r| r.present).count();
                            self.status = format!(
                                "Gereed: {totaal} frames, Aanwezig: {aanwezig} ({:.1?})",
                                dur
                            );
                            self.rijen = rows;
                            self.thumbs.clear();
                            self.thumb_keys.clear();
                        }
                        Err(e) => {
                            self.status = format!("Fout bij scannen: {e}");
                        }
                    }
                    self.bezig = false;
                }

                let kan_exporteren = !self.rijen.is_empty() && !self.bezig;
                if ui
                    .add_enabled(kan_exporteren, egui::Button::new("Exporteer CSV"))
                    .clicked()
                    && let Some(path) = FileDialog::new()
                        .add_filter("CSV", &["csv"])
                        .set_file_name("feeder_vision.csv")
                        .save_file()
                {
                    if let Err(e) = export_csv(&self.rijen, &path) {
                        self.status = format!("Fout bij exporteren: {e}");
                    } else {
                        self.status = format!("CSV geëxporteerd: {}", path.display());
                    }
                }

                if !self.status.is_empty() {
                    ui.label(&self.status);
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.rijen.is_empty() && self.gekozen_map.is_some() && !self.bezig {
                ui.heading("Geen afbeeldingen gevonden");
            }

            if !self.rijen.is_empty() {
                let totaal = self.rijen.len();
                let aanwezig = self.rijen.iter().filter(|r| r.present).count();
                ui.label(format!("Totaal: {totaal} — Aanwezig: {aanwezig}"));

                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            let thumb_px = THUMB_SIZE as f32;
                            let desired = egui::Vec2::new(thumb_px, thumb_px);

                            for i in 0..self.rijen.len() {
                                let path = self.rijen[i].file.clone();
                                let (resp, painter) =
                                    ui.allocate_painter(desired, egui::Sense::hover());
                                let r = resp.rect;
                                if let Some(id) = self.get_or_load_thumb(ctx, &path) {
                                    let uv = egui::Rect::from_min_max(
                                        egui::pos2(0.0, 0.0),
                                        egui::pos2(1.0, 1.0),
                                    );
                                    painter.image(id, uv, r, egui::Color32::WHITE);
                                } else {
                                    painter.rect_filled(r, 4.0, egui::Color32::from_gray(40));
                                    painter.rect_stroke(
                                        r,
                                        4.0,
                                        egui::Stroke::new(1.0, egui::Color32::DARK_GRAY),
                                        egui::StrokeKind::Inside,
                                    );
                                }
                            }
                        });
                    });
            }
        });
    }
}
