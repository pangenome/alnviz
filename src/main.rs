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
    view_history: Vec<ViewState>,  // For 'z' key to go back
    needs_initial_fit: bool,        // Flag to fit view on first render
    last_canvas_size: (f32, f32),   // Last canvas dimensions for zoom limits

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
    box_zoom_start: Option<egui::Pos2>,  // Shift+drag box zoom
    selected_segment: Option<usize>,     // For x/X key selection
}

#[derive(Clone)]
enum LoadingState {
    Idle,
    Loading(String), // file path
    Success(String),
    Failed(String),
}

#[derive(Clone)]
struct ViewState {
    x: f64,          // Genome x coordinate at left edge
    y: f64,          // Genome y coordinate at bottom edge
    scale: f64,      // Base pairs per pixel

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
                scale: 1000.0,  // 1000 bp per pixel initially
                max_x: 1_000_000.0,
                max_y: 1_000_000.0,
            },
            view_history: Vec::new(),
            needs_initial_fit: false,
            last_canvas_size: (800.0, 600.0),
            layers: vec![LayerSettings::default()],
            num_layers: 0,
            current_file: None,
            show_about: false,
            loading: Arc::new(Mutex::new(LoadingState::Idle)),
            plot_receiver: None,
            box_zoom_start: None,
            selected_segment: None,
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
                            // Extract real genome lengths
                            let alen = safe_plot.get_alen() as f64;
                            let blen = safe_plot.get_blen() as f64;
                            println!("✅ Plot loaded successfully! Genome lengths: {} x {}", alen, blen);

                            // Update view with actual genome dimensions
                            self.view.max_x = alen;
                            self.view.max_y = blen;
                            self.view.x = 0.0;
                            self.view.y = 0.0;
                            // Will fit to canvas on first render
                            self.needs_initial_fit = true;

                            // Get actual number of layers from plot
                            let nlays = safe_plot.get_nlays() as usize;
                            println!("  Plot has {} layers", nlays);

                            self.plot = Some(safe_plot);
                            self.num_layers = nlays;

                            // Create layer settings for all layers
                            self.layers = (0..nlays).map(|i| LayerSettings {
                                visible: true,
                                name: format!("Layer {}", i),
                                ..Default::default()
                            }).collect();

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
                ui.label(format!("Scale: {:.1} bp/px", self.view.scale));
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
                        "Pos: X={:.0} Y={:.0}  Scale: {:.1} bp/px",
                        self.view.x,
                        self.view.y,
                        self.view.scale
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

        // Track canvas size for zoom limits
        self.last_canvas_size = (rect.width(), rect.height());

        // Fit view to canvas on first render after loading
        if self.needs_initial_fit && rect.width() > 0.0 && rect.height() > 0.0 {
            self.fit_view_to_canvas(rect);
            self.needs_initial_fit = false;
        }

        // Handle interaction
        self.handle_interaction(&response, rect);

        // Genome to screen mapping using scale (bp/pixel)
        let genome_to_screen = |gx: f64, gy: f64| -> egui::Pos2 {
            let pixel_x = (gx - self.view.x) / self.view.scale;
            let pixel_y = (gy - self.view.y) / self.view.scale;

            egui::pos2(
                rect.min.x + pixel_x as f32,
                rect.max.y - pixel_y as f32, // Y is flipped
            )
        };

        // Background - black like ALNVIEW
        painter.rect_filled(rect, 0.0, egui::Color32::BLACK);

        // Draw genome boundaries and scaffold lines
        if let Some(ref plot) = self.plot {
            let alen = plot.get_alen() as f64;
            let blen = plot.get_blen() as f64;

            // Calculate visible genome region
            let view_width = rect.width() as f64 * self.view.scale;
            let view_height = rect.height() as f64 * self.view.scale;

            // Draw scaffold boundaries for genome A (vertical dashed gray lines)
            let scaffolds_a = plot.get_scaffold_boundaries(0);
            for &pos in &scaffolds_a {
                let x = pos as f64;
                if x >= self.view.x && x <= self.view.x + view_width {
                    let x_pos = genome_to_screen(x, 0.0).x;
                    // TODO: egui doesn't support dashed lines yet, using solid gray
                    painter.vline(x_pos, rect.y_range(), (1.0, egui::Color32::from_rgb(100, 100, 100)));
                }
            }

            // Draw scaffold boundaries for genome B (horizontal dashed gray lines)
            let scaffolds_b = plot.get_scaffold_boundaries(1);
            for &pos in &scaffolds_b {
                let y = pos as f64;
                if y >= self.view.y && y <= self.view.y + view_height {
                    let y_pos = genome_to_screen(0.0, y).y;
                    painter.hline(rect.x_range(), y_pos, (1.0, egui::Color32::from_rgb(100, 100, 100)));
                }
            }

            // Draw genome end boundaries (thicker)
            if alen >= self.view.x && alen <= self.view.x + view_width {
                let x_pos = genome_to_screen(alen, 0.0).x;
                painter.vline(x_pos, rect.y_range(), (2.0, egui::Color32::DARK_RED));
            }

            if blen >= self.view.y && blen <= self.view.y + view_height {
                let y_pos = genome_to_screen(0.0, blen).y;
                painter.hline(rect.x_range(), y_pos, (2.0, egui::Color32::DARK_BLUE));
            }

            // Draw axes at origin
            if self.view.x <= 0.0 && self.view.x + view_width >= 0.0 {
                let x_pos = genome_to_screen(0.0, 0.0).x;
                painter.vline(x_pos, rect.y_range(), (1.0, egui::Color32::GRAY));
            }
            if self.view.y <= 0.0 && self.view.y + view_height >= 0.0 {
                let y_pos = genome_to_screen(0.0, 0.0).y;
                painter.hline(rect.x_range(), y_pos, (1.0, egui::Color32::GRAY));
            }
        }

        // Draw alignment segments for each visible layer
        if let Some(ref plot) = self.plot {
            for (layer_idx, layer_settings) in self.layers.iter().enumerate() {
                if !layer_settings.visible || layer_idx >= self.num_layers {
                    continue;
                }

                // Calculate visible genome region based on canvas size and scale
                let view_width = rect.width() as f64 * self.view.scale;
                let view_height = rect.height() as f64 * self.view.scale;

                // Query R*-tree for segments in visible region
                let visible_segs = plot.query_segments_in_region(
                    layer_idx as i32,
                    self.view.x,
                    self.view.y,
                    view_width,
                    view_height,
                );

                // Draw visible segments
                for seg in visible_segs {

                    // Draw the segment as a line
                    let p1 = genome_to_screen(seg.abeg as f64, seg.bbeg as f64);
                    let p2 = genome_to_screen(seg.aend as f64, seg.bend as f64);

                    // Forward (positive slope) = green, Reverse (negative slope) = red
                    let is_forward = (seg.bend - seg.bbeg) * (seg.aend - seg.abeg) > 0;
                    let color = if is_forward {
                        egui::Color32::from_rgb(0, 255, 0)  // Green for forward
                    } else {
                        egui::Color32::from_rgb(255, 0, 0)  // Red for reverse complement
                    };

                    painter.line_segment(
                        [p1, p2],
                        egui::Stroke::new(1.0, color),
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
        let view_width = rect.width() as f64 * self.view.scale;
        let view_height = rect.height() as f64 * self.view.scale;

        // X axis label
        let x_text = format!("{:.0} - {:.0} bp", self.view.x, self.view.x + view_width);
        painter.text(
            egui::pos2(rect.center().x, rect.max.y - 5.0),
            egui::Align2::CENTER_BOTTOM,
            x_text,
            egui::FontId::proportional(10.0),
            egui::Color32::DARK_GRAY,
        );

        // Y axis label (rotated would be nice, but keeping simple for now)
        let y_text = format!("{:.0} - {:.0} bp", self.view.y, self.view.y + view_height);
        painter.text(
            egui::pos2(rect.min.x + 5.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            y_text,
            egui::FontId::proportional(10.0),
            egui::Color32::DARK_GRAY,
        );
    }

    fn handle_interaction(&mut self, response: &egui::Response, rect: egui::Rect) {
        // Z key - go back in zoom history
        response.ctx.input(|i| {
            if i.key_pressed(egui::Key::Z) {
                if let Some(prev_view) = self.view_history.pop() {
                    self.view = prev_view;
                }
            }
        });

        // Shift+drag for box zoom
        if response.hovered() {
            let shift_held = response.ctx.input(|i| i.modifiers.shift);

            if shift_held && response.drag_started() {
                self.box_zoom_start = response.hover_pos();
            }

            if let Some(start) = self.box_zoom_start {
                if response.dragged() {
                    // Draw box while dragging
                    if let Some(current) = response.hover_pos() {
                        let painter = response.ctx.debug_painter();
                        let box_rect = egui::Rect::from_two_pos(start, current);
                        painter.rect_stroke(box_rect, 0.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 100, 100)));
                    }
                }

                if response.drag_stopped() {
                    // Zoom to box
                    if let Some(end) = response.hover_pos() {
                        self.zoom_to_box(rect, start, end);
                    }
                    self.box_zoom_start = None;
                }
            }
        }

        // Regular pan on drag (when shift not held)
        if response.dragged() && !response.ctx.input(|i| i.modifiers.shift) {
            let delta = response.drag_delta();
            let dx = -delta.x as f64 * self.view.scale;
            let dy = delta.y as f64 * self.view.scale;

            let view_width = rect.width() as f64 * self.view.scale;
            let view_height = rect.height() as f64 * self.view.scale;

            // Clamp to genome bounds (0,0) to (max_x, max_y)
            // When zoomed out, this prevents panning beyond genome edges
            self.view.x = (self.view.x + dx).max(0.0).min((self.view.max_x - view_width).max(0.0));
            self.view.y = (self.view.y + dy).max(0.0).min((self.view.max_y - view_height).max(0.0));
        }

        // Scroll wheel zoom
        if response.hovered() {
            let scroll = response.ctx.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                let zoom_factor = if scroll > 0.0 { 1.2 } else { 0.8 };
                if let Some(mouse_pos) = response.hover_pos() {
                    self.zoom_at_point(zoom_factor, mouse_pos, rect);
                } else {
                    self.zoom(zoom_factor);
                }
            }
        }
    }

    fn zoom_to_box(&mut self, canvas_rect: egui::Rect, screen_start: egui::Pos2, screen_end: egui::Pos2) {
        // Convert screen coordinates to genome coordinates
        let screen_to_genome = |pos: egui::Pos2| -> (f64, f64) {
            let pixel_x = (pos.x - canvas_rect.min.x) as f64;
            let pixel_y = (canvas_rect.max.y - pos.y) as f64;

            let gx = self.view.x + pixel_x * self.view.scale;
            let gy = self.view.y + pixel_y * self.view.scale;
            (gx, gy)
        };

        let (x1, y1) = screen_to_genome(screen_start);
        let (x2, y2) = screen_to_genome(screen_end);

        let min_x = x1.min(x2);
        let max_x = x1.max(x2);
        let min_y = y1.min(y2);
        let max_y = y1.max(y2);

        let box_width = max_x - min_x;
        let box_height = max_y - min_y;

        // Save current view to history
        self.view_history.push(self.view.clone());

        // Set new view position
        self.view.x = min_x.max(0.0);
        self.view.y = min_y.max(0.0);

        // Calculate new scale to fit the box in the canvas
        let scale_for_width = box_width / canvas_rect.width() as f64;
        let scale_for_height = box_height / canvas_rect.height() as f64;
        self.view.scale = scale_for_width.max(scale_for_height).max(0.1);

        // Clamp position (allow zooming out beyond genome bounds)
        self.view.x = self.view.x.max(0.0);
        self.view.y = self.view.y.max(0.0);
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
    fn fit_view_to_canvas(&mut self, canvas_rect: egui::Rect) {
        // Calculate scale to fit smaller dimension exactly (user can scroll for the longer one)
        let scale_x = self.view.max_x / canvas_rect.width() as f64;
        let scale_y = self.view.max_y / canvas_rect.height() as f64;
        self.view.scale = scale_x.min(scale_y);
        self.view.x = 0.0;
        self.view.y = 0.0;
    }

    fn zoom(&mut self, factor: f64) {
        // Calculate new scale
        let new_scale = self.view.scale / factor;

        // Don't zoom out beyond where smaller dimension fills the window
        // (higher scale = more zoomed out = more bp per pixel)
        let max_scale_x = self.view.max_x / self.last_canvas_size.0 as f64;
        let max_scale_y = self.view.max_y / self.last_canvas_size.1 as f64;
        let max_scale = max_scale_x.min(max_scale_y);

        // Apply zoom with limit: don't zoom out too far
        self.view.scale = new_scale.min(max_scale);
    }

    fn zoom_at_point(&mut self, factor: f64, screen_pos: egui::Pos2, canvas_rect: egui::Rect) {
        // Convert screen position to genome coordinates
        let pixel_x = (screen_pos.x - canvas_rect.min.x) as f64;
        let pixel_y = (canvas_rect.max.y - screen_pos.y) as f64;

        let genome_x = self.view.x + pixel_x * self.view.scale;
        let genome_y = self.view.y + pixel_y * self.view.scale;

        // Calculate new scale
        let new_scale = self.view.scale / factor;

        // Don't zoom out beyond where smaller dimension fills the window
        // (higher scale = more zoomed out = more bp per pixel)
        let max_scale_x = self.view.max_x / canvas_rect.width() as f64;
        let max_scale_y = self.view.max_y / canvas_rect.height() as f64;
        let max_scale = max_scale_x.min(max_scale_y);

        // Apply zoom with limit: don't zoom out too far
        self.view.scale = new_scale.min(max_scale);

        // Keep the mouse position at the same genome coordinate
        self.view.x = genome_x - pixel_x * self.view.scale;
        self.view.y = genome_y - pixel_y * self.view.scale;

        // Clamp position to prevent panning outside genome bounds
        let view_width = canvas_rect.width() as f64 * self.view.scale;
        let view_height = canvas_rect.height() as f64 * self.view.scale;

        // Clamp to genome bounds (handle both zoomed in and zoomed out)
        self.view.x = self.view.x.max(0.0).min((self.view.max_x - view_width).max(0.0));
        self.view.y = self.view.y.max(0.0).min((self.view.max_y - view_height).max(0.0));
    }

    fn reset_view(&mut self) {
        self.needs_initial_fit = true;
    }
}
