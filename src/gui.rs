use std::fs;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use directories::UserDirs;
use eframe::egui;
use tempfile::{Builder, NamedTempFile};

use monolith::cache::Cache;
use monolith::core::{
    MonolithOptions, MonolithOutputFormat, create_monolithic_document, format_output_path,
};
use monolith::session::Session;

const CACHE_ASSET_FILE_SIZE_THRESHOLD: usize = 1024 * 20; // Minimum file size for on-disk caching (in bytes)
const DEFAULT_OUTPUT_FILE_NAME: &str = "%title% - %timestamp%.%extension%";
const GRID_SPACING: f32 = 6.0;
const MIN_FIELD_WIDTH: f32 = 120.0;
const SEPARATOR_WIDTH: f32 = 6.0; // egui's default separator spacing
const WINDOW_ICON: &[u8] = include_bytes!("../assets/icon/icon-128.png");
const WINDOW_SIZE: [f32; 2] = [760.0, 320.0];
const WINDOW_MIN_SIZE: [f32; 2] = [600.0, 320.0];

/// Outcome of a background job: path of the written file, or an error message
type JobResult = Result<String, String>;

#[derive(Clone, Copy, PartialEq)]
enum OutputFormat {
    Html,
    Mhtml,
}

impl OutputFormat {
    const ALL: [OutputFormat; 2] = [OutputFormat::Html, OutputFormat::Mhtml];

    fn label(self) -> &'static str {
        match self {
            OutputFormat::Html => "HTML",
            OutputFormat::Mhtml => "MHTML",
        }
    }

    fn to_monolith(self) -> MonolithOutputFormat {
        match self {
            OutputFormat::Html => MonolithOutputFormat::HTML,
            OutputFormat::Mhtml => MonolithOutputFormat::MHTML,
        }
    }
}

struct MonolithApp {
    target: String,
    output_path: String,
    output_format: OutputFormat,
    keep_fonts: bool,
    keep_frames: bool,
    keep_images: bool,
    keep_scripts: bool,
    keep_styles: bool,
    isolate: bool,
    unwrap_noscript: bool,
    ignore_errors: bool,
    job: Option<Receiver<JobResult>>, // Some while a page is being saved
    status: String,
    options_width: f32, // Width of the right column, as measured during the previous frame
}

impl Default for MonolithApp {
    fn default() -> Self {
        Self {
            target: String::new(),
            output_path: default_output_path(),
            output_format: OutputFormat::Html,
            keep_fonts: false,
            keep_frames: true,
            keep_images: true,
            keep_scripts: true,
            keep_styles: true,
            isolate: true,
            unwrap_noscript: false,
            ignore_errors: false,
            job: None,
            status: String::new(),
            options_width: 160.0,
        }
    }
}

fn main() -> eframe::Result {
    let mut program_name: String = env!("CARGO_PKG_NAME").to_string();
    if let Some(l) = program_name.get_mut(0..1) {
        l.make_ascii_uppercase();
    }

    let mut viewport = egui::ViewportBuilder::default()
        .with_title(program_name.clone())
        .with_inner_size(WINDOW_SIZE)
        .with_min_inner_size(WINDOW_MIN_SIZE);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(WINDOW_ICON) {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        &program_name,
        options,
        Box::new(|_cc| Ok(Box::<MonolithApp>::default())),
    )
}

fn default_output_path() -> String {
    let user_dirs = UserDirs::new();
    match user_dirs.as_ref().and_then(|dirs| dirs.download_dir()) {
        Some(dir) => dir.join(DEFAULT_OUTPUT_FILE_NAME).display().to_string(),
        None => DEFAULT_OUTPUT_FILE_NAME.to_string(),
    }
}

/// Runs on the worker thread: build the document and write it to disk
fn save_document(
    session: Session,
    target: String,
    output_path: &str,
    format: OutputFormat,
) -> JobResult {
    let (data, title) = create_monolithic_document(session, target).map_err(|e| e.to_string())?;
    let path = format_output_path(
        output_path,
        &title.unwrap_or_default(),
        format.to_monolith(),
    );

    // Always create missing directories, GUI users shouldn't have to know they need to exist
    if let Some(dir) = Path::new(&path)
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
    {
        fs::create_dir_all(dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    }

    fs::write(&path, data).map_err(|e| format!("could not write {path}: {e}"))?;
    Ok(path)
}

/// Width for the path text fields, so that label + field + button fill the available width
fn path_field_width(ui: &egui::Ui) -> f32 {
    let measure = |text: &str, style: egui::TextStyle| {
        let font = style.resolve(ui.style());
        ui.painter()
            .layout_no_wrap(text.to_string(), font, egui::Color32::WHITE)
            .size()
            .x
    };
    let label_width = measure("Target:", egui::TextStyle::Body)
        .max(measure("Output path:", egui::TextStyle::Body));
    let button_width = measure("Open file", egui::TextStyle::Button)
        .max(measure("Browse", egui::TextStyle::Button))
        + 2.0 * ui.spacing().button_padding.x;

    (ui.available_width() - label_width - button_width - 2.0 * GRID_SPACING - 1.0)
        .max(MIN_FIELD_WIDTH)
}

impl MonolithApp {
    fn start(&mut self, ctx: &egui::Context) {
        let mut options: MonolithOptions = MonolithOptions::default();
        options.ignore_errors = self.ignore_errors;
        options.insecure = true;
        options.silent = true;
        options.no_frames = !self.keep_frames;
        options.no_fonts = !self.keep_fonts;
        options.no_images = !self.keep_images;
        options.no_css = !self.keep_styles;
        options.no_js = !self.keep_scripts;
        options.isolate = self.isolate;
        options.threads = 30;
        options.unwrap_noscript = self.unwrap_noscript;
        options.output_format = self.output_format.to_monolith();

        let target = self.target.clone();
        let output_path = self.output_path.clone();
        let format = self.output_format;
        let ctx = ctx.clone();
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            // Set up cache (attempt to create temporary file)
            let temp_cache_file: Option<NamedTempFile> =
                Builder::new().prefix("monolith-").tempfile().ok();
            let cache = Cache::new(
                CACHE_ASSET_FILE_SIZE_THRESHOLD,
                temp_cache_file
                    .as_ref()
                    .map(|file| file.path().display().to_string()),
            );
            let session: Session = Session::new(Some(cache), None, options);

            let result = save_document(session, target, &output_path, format);

            // The session (and its cache database) is gone by now,
            // so dropping the temp file removes the database from disk
            drop(temp_cache_file);

            let _ = tx.send(result);
            ctx.request_repaint(); // Wake the UI up to pick up the result
        });

        self.job = Some(rx);
        self.status.clear();
    }

    fn poll_job(&mut self) {
        let Some(received) = self.job.as_ref().map(|rx| rx.try_recv()) else {
            return;
        };

        match received {
            Err(TryRecvError::Empty) => return, // Still working
            Ok(Ok(path)) => self.status = format!("Saved to {path}"),
            Ok(Err(error)) => self.status = format!("Error: {error}"),
            Err(TryRecvError::Disconnected) => {
                self.status = "Error: worker thread stopped unexpectedly".to_string()
            }
        }
        self.job = None;
    }

    fn browse_target(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("HTML files", &["html", "htm"])
            .pick_file()
        {
            self.target = path.display().to_string();
        }
    }

    fn browse_output_path(&mut self) {
        let current = Path::new(&self.output_path);
        let mut dialog = rfd::FileDialog::new();
        if let Some(name) = current.file_name() {
            dialog = dialog.set_file_name(name.to_string_lossy());
        }
        if let Some(dir) = current.parent().filter(|dir| dir.is_dir()) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.save_file() {
            self.output_path = path.display().to_string();
        }
    }

    /// Left column: target, output path, Start button, status
    fn main_column(&mut self, ui: &mut egui::Ui, busy: bool) {
        let field_width = path_field_width(ui);

        ui.add_enabled_ui(!busy, |ui| {
            egui::Grid::new("paths")
                .num_columns(3)
                .spacing([GRID_SPACING, GRID_SPACING])
                .show(ui, |ui| {
                    ui.label("Target:");
                    ui.add_sized(
                        [field_width, 20.0],
                        egui::TextEdit::singleline(&mut self.target)
                            .hint_text("Paste URL or file path")
                            .desired_width(field_width),
                    );
                    if ui.button("Open file").clicked() {
                        self.browse_target();
                    }
                    ui.end_row();

                    ui.label("Output path:");
                    ui.add_sized(
                        [field_width, 20.0],
                        egui::TextEdit::singleline(&mut self.output_path)
                            .hint_text("Filesystem path")
                            .desired_width(field_width),
                    );
                    if ui.button("Browse").clicked() {
                        self.browse_output_path();
                    }
                    ui.end_row();
                });
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let can_start = !busy && !self.target.is_empty() && !self.output_path.is_empty();
            if ui
                .add_enabled(can_start, egui::Button::new("Start"))
                .clicked()
            {
                let ctx = ui.ctx().clone();
                self.start(&ctx);
            }
            if busy {
                ui.spinner();
            }
        });

        ui.add_space(6.0);
        if !busy && !self.status.is_empty() {
            ui.label(&self.status);
        }
    }

    /// Right column: options
    fn options_column(&mut self, ui: &mut egui::Ui) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend); // Never wrap labels here

        ui.checkbox(&mut self.keep_fonts, "Include fonts");
        ui.checkbox(&mut self.keep_frames, "Include (i)frames");
        ui.checkbox(&mut self.keep_images, "Include images");
        ui.checkbox(&mut self.keep_scripts, "Include scripts");
        ui.checkbox(&mut self.keep_styles, "Include styles");

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Output format:");
            egui::ComboBox::from_id_salt("output_format")
                .selected_text(self.output_format.label())
                .show_ui(ui, |ui| {
                    for format in OutputFormat::ALL {
                        ui.selectable_value(&mut self.output_format, format, format.label());
                    }
                });
        });

        ui.add_space(6.0);
        ui.checkbox(&mut self.unwrap_noscript, "Unwrap NOSCRIPT");
        ui.checkbox(&mut self.isolate, "Isolate document");
        ui.checkbox(&mut self.ignore_errors, "Ignore network errors");
    }
}

impl eframe::App for MonolithApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_job();
        let busy = self.job.is_some();

        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal_top(|ui| {
                // Left column gets whatever the options column doesn't need
                let gap = 2.0 * ui.spacing().item_spacing.x + SEPARATOR_WIDTH;
                let left_width = (ui.available_width() - self.options_width - gap).max(0.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(left_width, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_min_width(left_width);
                        self.main_column(ui, busy);
                    },
                );

                ui.separator();

                let options = ui.vertical(|ui| {
                    ui.add_enabled_ui(!busy, |ui| self.options_column(ui));
                });

                // Options column is sized to its content; re-layout if that changed
                let measured = options.response.rect.width();
                if (measured - self.options_width).abs() > 0.5 {
                    self.options_width = measured;
                    ui.ctx().request_repaint();
                }
            });
        });
    }
}
