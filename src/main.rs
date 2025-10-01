mod ffi;

use eframe::egui;
use std::ffi::CString;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver};
use std::thread;

// Wrapper to make raw pointer Send (UNSAFE but necessary for FFI)
struct SendPtr(*mut ffi::DotPlot);
unsafe impl Send for SendPtr {}

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("ALNview - Rust Edition"),
        ..Default::default()
    };

    eframe::run_native(
        "ALNview",
        options,
        Box::new(|_cc| Ok(Box::new(AlnViewApp::default()))),
    )
}

// ============================================================================
// Application State
// ============================================================================

struct AlnViewApp {
    // Data
    plot: Option<ffi::SafePlot>,

    // View state
    view: ViewState,

    // Layer settings
    layers: Vec<LayerSettings>,
    num_layers: usize,

    // UI state
    current_file: Option<PathBuf>,
    show_about: bool,

    // Loading state
    loading: Arc<Mutex<LoadingState>>,
    plot_receiver: Option<Receiver<Result<SendPtr, String>>>,

    // Interaction state
    dragging: bool,
    drag_start: egui::Pos2,
}

#[derive(Clone)]
enum LoadingState {
    Idle,
    Loading(String), // file path
    Success(String),
    Failed(String),
}

struct ViewState {
    x: f64,
    y: f64,
    width: f64,
    height: f64,

    // Genome lengths (from plot)
    max_x: f64,
    max_y: f64,
}

#[derive(Clone)]
struct LayerSettings {
    visible: bool,
    name: String,
    color_forward: egui::Color32,
    color_reverse: egui::Color32,
    thickness: f32,
}

impl Default for AlnViewApp {
    fn default() -> Self {
        Self {
            plot: None,
            view: ViewState {
                x: 0.0,
                y: 0.0,
                width: 1_000_000.0,
                height: 1_000_000.0,
                max_x: 1_000_000.0,
                max_y: 1_000_000.0,
            },
            layers: vec![LayerSettings::default()],
            num_layers: 0,
            current_file: None,
            show_about: false,
            loading: Arc::new(Mutex::new(LoadingState::Idle)),
            plot_receiver: None,
            dragging: false,
            drag_start: egui::Pos2::ZERO,
        }
    }
}

impl Default for LayerSettings {
    fn default() -> Self {
        Self {
            visible: true,
            name: "Layer 0".to_string(),
            color_forward: egui::Color32::from_rgb(0, 100, 200),
            color_reverse: egui::Color32::from_rgb(200, 100, 0),
            thickness: 2.0,
        }
    }
}

// ============================================================================
// Main App Implementation
// ============================================================================

impl eframe::App for AlnViewApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check if plot loaded from background thread
        if let Some(ref receiver) = self.plot_receiver {
            if let Ok(result) = receiver.try_recv() {
                match result {
                    Ok(SendPtr(plot_ptr)) => {
                        if let Some(safe_plot) = ffi::SafePlot::new(plot_ptr) {
                            println!("✅ Plot loaded successfully!");
                            self.plot = Some(safe_plot);
                            self.num_layers = 1;
                            self.layers = vec![LayerSettings {
                                visible: true,
                                name: "Alignments".to_string(),
                                ..Default::default()
                            }];
                            // TODO: Get actual genome lengths
                            self.view.max_x = 100_000_000.0;
                            self.view.max_y = 100_000_000.0;
                            self.reset_view();

                            *self.loading.lock().unwrap() = LoadingState::Success("Loaded successfully".to_string());
                        } else {
                            *self.loading.lock().unwrap() = LoadingState::Failed("C returned NULL".to_string());
                        }
                    }
                    Err(e) => {
                        *self.loading.lock().unwrap() = LoadingState::Failed(e);
                    }
                }
                self.plot_receiver = None;
            }
        }

        // Check loading state
        let loading_state = self.loading.lock().unwrap().clone();
        match loading_state {
            LoadingState::Success(msg) => {
                println!("✅ {}", msg);
                *self.loading.lock().unwrap() = LoadingState::Idle;
            }
            LoadingState::Failed(msg) => {
                eprintln!("❌ {}", msg);
                *self.loading.lock().unwrap() = LoadingState::Idle;
            }
            _ => {}
        }

        // Menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("📁 Open .1aln file...").clicked() {
                        self.open_file_dialog();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("❌ Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("View", |ui| {
                    if ui.button("🔍 Zoom In").clicked() {
                        self.zoom(2.0);
                        ui.close_menu();
                    }
                    if ui.button("🔍 Zoom Out").clicked() {
                        self.zoom(0.5);
                        ui.close_menu();
                    }
                    if ui.button("🏠 Reset View").clicked() {
                        self.reset_view();
                        ui.close_menu();
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("ℹ About").clicked() {
                        self.show_about = true;
                        ui.close_menu();
                    }
                });

                ui.separator();

                // Quick zoom buttons
                if ui.button("🔍+").clicked() {
                    self.zoom(2.0);
                }
                if ui.button("🔍-").clicked() {
                    self.zoom(0.5);
                }
                if ui.button("🏠").clicked() {
                    self.reset_view();
                }
            });
        });

        // Side panel for layer controls
        egui::SidePanel::left("layers_panel")
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Layers");
                ui.separator();

                if self.num_layers == 0 {
                    ui.label("No layers loaded");
                } else {
                    for i in 0..self.num_layers {
                        if i < self.layers.len() {
                            self.layer_control(ui, i);
                            ui.separator();
                        }
                    }
                }

                ui.separator();
                ui.label(format!("View: {:.0} × {:.0}", self.view.width, self.view.height));
            });

        // Status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Show loading state
                match &*self.loading.lock().unwrap() {
                    LoadingState::Loading(path) => {
                        ui.spinner();
                        ui.label(format!("Loading: {}", path));
                    }
                    _ => {
                        if let Some(ref path) = self.current_file {
                            ui.label(format!("📄 {}", path.display()));
                        } else {
                            ui.label("No file loaded");
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "X: {:.0} - {:.0}  Y: {:.0} - {:.0}",
                        self.view.x,
                        self.view.x + self.view.width,
                        self.view.y,
                        self.view.y + self.view.height
                    ));
                });
            });
        });

        // Main canvas
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.plot.is_some() {
                self.render_canvas(ui);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("🦀 ALNview - Rust Edition");
                        ui.add_space(20.0);

                        let is_loading = matches!(&*self.loading.lock().unwrap(), LoadingState::Loading(_));

                        if is_loading {
                            if let LoadingState::Loading(path) = &*self.loading.lock().unwrap() {
                                ui.spinner();
                                ui.label(format!("Loading: {}...", path));
                                ui.label("This may take a while for large files");
                            }
                        } else {
                            ui.label("Open a .1aln file to begin");
                            ui.add_space(10.0);
                            if ui.button("📁 Open File").clicked() {
                                self.open_file_dialog();
                            }
                        }
                    });
                });
            }
        });

        // About dialog
        if self.show_about {
            egui::Window::new("About ALNview")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("ALNview - Rust Edition");
                    ui.separator();
                    ui.label("A Qt-free alignment viewer for FASTGA");
                    ui.add_space(10.0);
                    ui.label("Original author: Gene Myers");
                    ui.label("Rust port: 2025");
                    ui.add_space(10.0);
                    ui.label("Built with:");
                    ui.label("  • Rust 🦀");
                    ui.label("  • egui (immediate mode GUI)");
                    ui.label("  • C backend (temporary FFI)");
                    ui.add_space(10.0);
                    if ui.button("Close").clicked() {
                        self.show_about = false;
                    }
                });
        }

        // Request repaint if loading
        if matches!(&*self.loading.lock().unwrap(), LoadingState::Loading(_)) {
            ctx.request_repaint();
        }
    }
}

// ============================================================================
// UI Components
// ============================================================================

impl AlnViewApp {
    fn layer_control(&mut self, ui: &mut egui::Ui, idx: usize) {
        let layer = &mut self.layers[idx];

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut layer.visible, "");
                ui.strong(&layer.name);
            });

            ui.horizontal(|ui| {
                ui.label("Forward:");
                ui.color_edit_button_srgba(&mut layer.color_forward);
            });

            ui.horizontal(|ui| {
                ui.label("Reverse:");
                ui.color_edit_button_srgba(&mut layer.color_reverse);
            });

            ui.horizontal(|ui| {
                ui.label("Thickness:");
                ui.add(egui::Slider::new(&mut layer.thickness, 0.5..=10.0));
            });
        });
    }

    fn render_canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );

        let rect = response.rect;

        // Handle interaction
        self.handle_interaction(&response);

        // Coordinate transformation
        let _to_screen = |gx: f64, gy: f64| -> egui::Pos2 {
            let norm_x = (gx - self.view.x) / self.view.width;
            let norm_y = (gy - self.view.y) / self.view.height;

            egui::pos2(
                rect.min.x + norm_x as f32 * rect.width(),
                rect.max.y - norm_y as f32 * rect.height(), // Y is flipped
            )
        };

        // Background
        painter.rect_filled(rect, 0.0, egui::Color32::WHITE);

        // Draw alignment segments for each visible layer
        if let Some(ref plot) = self.plot {
            for (layer_idx, layer_settings) in self.layers.iter().enumerate() {
                if !layer_settings.visible || layer_idx >= self.num_layers {
                    continue;
                }

                let frame = ffi::Frame::new(
                    self.view.x,
                    self.view.y,
                    self.view.width,
                    self.view.height,
                );

                // Query C backend for segments in view
                if let Some(seg_list) = plot.query_layer(layer_idx as i32, &frame) {
                // For now, we'll draw dummy segments since we need to properly
                // access the segment data from C. This is a placeholder.

                let num_segs = seg_list.len();

                    // TODO: Actually get segment pointers from C and draw them
                    // For now, just show that we're querying successfully

                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("Layer {} has {} segments in view", layer_idx, num_segs),
                        egui::FontId::proportional(14.0),
                        egui::Color32::DARK_GRAY,
                    );
                }
            }
        }

        // Draw border
        painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::GRAY));

        // Draw scale/axes
        self.draw_axes(ui, &painter, rect);
    }

    fn draw_axes(&self, _ui: &mut egui::Ui, painter: &egui::Painter, rect: egui::Rect) {
        // X axis label
        let x_text = format!("{:.0} - {:.0} bp", self.view.x, self.view.x + self.view.width);
        painter.text(
            egui::pos2(rect.center().x, rect.max.y - 5.0),
            egui::Align2::CENTER_BOTTOM,
            x_text,
            egui::FontId::proportional(10.0),
            egui::Color32::DARK_GRAY,
        );

        // Y axis label (rotated would be nice, but keeping simple for now)
        let y_text = format!("{:.0} - {:.0} bp", self.view.y, self.view.y + self.view.height);
        painter.text(
            egui::pos2(rect.min.x + 5.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            y_text,
            egui::FontId::proportional(10.0),
            egui::Color32::DARK_GRAY,
        );
    }

    fn handle_interaction(&mut self, response: &egui::Response) {
        // Pan on drag
        if response.dragged() {
            let delta = response.drag_delta();
            let dx = -delta.x as f64 * self.view.width / response.rect.width() as f64;
            let dy = delta.y as f64 * self.view.height / response.rect.height() as f64;

            self.view.x = (self.view.x + dx).max(0.0).min(self.view.max_x - self.view.width);
            self.view.y = (self.view.y + dy).max(0.0).min(self.view.max_y - self.view.height);
        }

        // Zoom on scroll
        if response.hovered() {
            let scroll = response.ctx.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                let zoom_factor = if scroll > 0.0 { 1.2 } else { 0.8 };
                self.zoom(zoom_factor);
            }
        }
    }
}

// ============================================================================
// File Operations
// ============================================================================

impl AlnViewApp {
    fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Alignment Files", &["1aln"])
            .pick_file()
        {
            self.load_file_async(path);
        }
    }

    fn load_file_async(&mut self, path: PathBuf) {
        let loading = Arc::clone(&self.loading);

        // Set loading state
        *loading.lock().unwrap() = LoadingState::Loading(
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file")
                .to_string()
        );

        println!("🔍 Starting async load: {}", path.display());

        let path_str = match path.to_str() {
            Some(s) => s.to_string(),
            None => {
                *loading.lock().unwrap() = LoadingState::Failed("Invalid path encoding".to_string());
                return;
            }
        };

        // Create channel for receiving plot
        let (tx, rx) = channel();
        self.plot_receiver = Some(rx);
        self.current_file = Some(path);

        // Spawn background thread for loading
        thread::spawn(move || {
            println!("🧵 Background thread: Loading file...");

            let c_path = match CString::new(path_str.as_str()) {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.send(Err(format!("Path contains null byte: {}", e)));
                    *loading.lock().unwrap() = LoadingState::Failed("Path contains null byte".to_string());
                    return;
                }
            };

            println!("📞 Calling C createPlot()...");

            let plot_ptr = unsafe {
                ffi::createPlot(
                    c_path.as_ptr(),
                    0,  // lCut
                    0,  // iCut
                    0,  // sCut
                    std::ptr::null_mut(),
                )
            };

            println!("📞 C createPlot() returned: {:?}", plot_ptr);

            if plot_ptr.is_null() {
                let _ = tx.send(Err(format!("Failed to load: {} (C returned NULL)", path_str)));
            } else {
                println!("✅ Sending plot to main thread via channel");
                let _ = tx.send(Ok(SendPtr(plot_ptr)));
            }
        });
    }
}

// ============================================================================
// View Operations
// ============================================================================

impl AlnViewApp {
    fn zoom(&mut self, factor: f64) {
        let center_x = self.view.x + self.view.width / 2.0;
        let center_y = self.view.y + self.view.height / 2.0;

        self.view.width /= factor;
        self.view.height /= factor;

        // Clamp to reasonable sizes
        self.view.width = self.view.width.max(100.0).min(self.view.max_x);
        self.view.height = self.view.height.max(100.0).min(self.view.max_y);

        // Recenter
        self.view.x = (center_x - self.view.width / 2.0).max(0.0);
        self.view.y = (center_y - self.view.height / 2.0).max(0.0);

        // Clamp position
        self.view.x = self.view.x.min(self.view.max_x - self.view.width);
        self.view.y = self.view.y.min(self.view.max_y - self.view.height);
    }

    fn reset_view(&mut self) {
        self.view.x = 0.0;
        self.view.y = 0.0;
        self.view.width = self.view.max_x;
        self.view.height = self.view.max_y;
    }
}
