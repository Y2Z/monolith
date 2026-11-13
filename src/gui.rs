use std::fs;
use std::io::{BufWriter, Write};
use std::path::{MAIN_SEPARATOR, Path, PathBuf};
use std::process::{self, Command};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::Duration;

use directories::UserDirs;
use eframe::egui;
use encoding_rs::Encoding;
use tempfile::{Builder, NamedTempFile};

#[cfg(feature = "browser")]
use monolith::browser;
use monolith::cache::Cache;
use monolith::core::{
    MonolithOptions, MonolithOutputFormat, create_monolithic_document, create_output_file,
    format_output_path, output_file_pattern, parse_target,
};
use monolith::net::PrefetchProgress;
use monolith::session::Session;
use monolith::url::Url;

const BORDER_GRAY: u8 = 0x22;
const BUTTON_ACTIVE_GRAY: u8 = 0xBB; // Fill of pressed buttons
const BUTTON_DISABLED_GRAY: u8 = 0x22;
const BUTTON_HOVER_GRAY: u8 = 0xDD; // Fill of buttons under the mouse pointer
const INTERRUPTED_EXIT_CODE: i32 = 130; // What shells report for programs ended with Ctrl+C
const CACHE_ASSET_FILE_SIZE_THRESHOLD: usize = 1024 * 20; // Minimum file size for on-disk caching (in bytes)
const CHECKBOX_GAP: f32 = 8.0; // Distance between a checkbox and its label
const CHECKBOX_MARK_INSET: f32 = 3.5;
const CHECKBOX_OUTLINE_GRAY: u8 = 0x55;
const CHECKBOX_ROW_HEIGHT: f32 = 18.0;
const CHECKBOX_ROW_SPACING: f32 = 3.0;
const CHECKBOX_SIZE: f32 = 13.0;
const CORNER_RADIUS: u8 = 5;
const DEFAULT_OUTPUT_FILE_NAME: &str = "%title% - %timestamp%.%extension%";
// Sorted, except for the first one; UTF-16 isn't here because encoding_rs can't write it
const ORIGINAL_ENCODING_LABEL: &str = "Original charset";
const OUTPUT_ENCODINGS: &[&str] = &[
    "Big5",
    "EUC-JP",
    "EUC-KR",
    "GB18030",
    "GBK",
    "ISO-2022-JP",
    "ISO-8859-1",
    "ISO-8859-2",
    "ISO-8859-5",
    "ISO-8859-6",
    "ISO-8859-7",
    "ISO-8859-8",
    "KOI8-R",
    "KOI8-U",
    "Shift_JIS",
    "UTF-8",
    "windows-1250",
    "windows-1251",
    "windows-1252",
    "windows-1253",
    "windows-1254",
    "windows-1255",
    "windows-1256",
    "windows-1257",
    "windows-1258",
    "windows-874",
];
const ENCODING_LIST_HEIGHT: f32 = 150.0; // Scrolls instead of leaving the fixed-size window
const FONT_SIZE: f32 = 14.0;
const FOCUS_GRAY: u8 = 0xCC; // Border of the text field that's being typed into
const FORMAT_BUTTON_WIDTH: f32 = 72.0;
const FORMAT_COLUMN_SPACING: f32 = 6.0;
const GRID_SPACING: f32 = 6.0;
const HEADING_FONT_SIZE: f32 = 20.0;
const HIGHLIGHT_GRAY: u8 = 80; // Fill of selected text
const INPUT_GRAY: u8 = 0x11;
const INPUT_MARGIN: egui::Margin = egui::Margin::symmetric(6, 2);
const MIN_FIELD_WIDTH: f32 = 120.0;
const MUTED_GRAY: u8 = 0x8A; // Row labels and checkboxes that are turned off
const POINTER_CURSOR: egui::CursorIcon = egui::CursorIcon::PointingHand;
const PROGRESS_BAR_HEIGHT: f32 = 20.0;
const PROGRESS_REFRESH_INTERVAL: Duration = Duration::from_millis(100);
const ROW_HEIGHT: f32 = 24.0;
const ROW_SPACING: f32 = 7.0; // Vertical gap between the rows of the window
const SAVE_TO_BUTTON: &str = "Browse";
const SAVE_TO_LABEL: &str = "Save to:";
const SPINNER_GAP: f32 = 8.0; // Distance between the spinner and the label of the Save button
const SPINNER_SIZE: f32 = 16.0;
const START_BUTTON_HEIGHT: f32 = 44.0;
const TARGET_BUTTON: &str = "Open file";
const TARGET_LABEL: &str = "Target:";
const TITLE_BAR_BUTTON_SIZE: [f32; 2] = [26.0, 24.0];
const TITLE_BAR_ICON_EXTENT: f32 = 5.5; // Half the width of the icons, stroke and blur included
const TITLE_BAR_HEIGHT: f32 = 28.0;
// Line segments that make up the title bar icons, as [x1, y1, x2, y2] offsets from their center
const CLOSE_ICON: &[[f32; 4]] = &[[-4.0, -4.0, 4.0, 4.0], [-4.0, 4.0, 4.0, -4.0]];
const MINIMIZE_ICON: &[[f32; 4]] = &[[-4.0, 3.0, 4.0, 3.0]];
// Radius of native windows; there's no way to ask the OS for it, these are the stock values
#[cfg(target_os = "macos")]
const WINDOW_CORNER_RADIUS: u8 = 10;
#[cfg(target_os = "windows")]
const WINDOW_CORNER_RADIUS: u8 = 8;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const WINDOW_CORNER_RADIUS: u8 = 12;
const TIMEOUT_DEFAULT: u64 = 120; // Same as the CLI
const TIMEOUT_MAX: u64 = 600; // What Session falls back to, also the most that can be typed in
const TIMEOUT_MIN: u64 = 1; // Session treats 0 as the maximum, which isn't what it looks like
const TIMEOUT_SLIDER_MAX: u64 = 120; // Anything above that is rarely needed, and can be typed in
const TIMEOUT_VALUE_WIDTH: f32 = 60.0; // Room for the value, so that it doesn't move the slider
const WINDOW_ICON: &[u8] = include_bytes!("../assets/icon/icon-128.png");
const WINDOW_MARGIN: egui::Margin = egui::Margin {
    left: 8,
    right: 8,
    top: 2,
    bottom: 6,
};
const WINDOW_SIZE: [f32; 2] = [716.0, 316.0]; // Roughly 9:4, proportions of the monolith

/// Outcome of a background job: path of the written file, along with
/// the encoding used (not known for files that get saved as they are), or an error message
type JobResult = Result<(String, Option<String>), String>;

#[derive(Clone, Copy, PartialEq)]
enum OutputFormat {
    Html,
    Mhtml,
    Warc,
    WarcGz,
}

impl OutputFormat {
    const ALL: [OutputFormat; 4] = [
        OutputFormat::Html,
        OutputFormat::Mhtml,
        OutputFormat::Warc,
        OutputFormat::WarcGz,
    ];

    fn label(self) -> &'static str {
        match self {
            OutputFormat::Html => "HTML",
            OutputFormat::Mhtml => "MHTML",
            OutputFormat::Warc => "WARC",
            OutputFormat::WarcGz => "WARCGZ",
        }
    }

    fn to_monolith(self) -> MonolithOutputFormat {
        match self {
            OutputFormat::Html => MonolithOutputFormat::HTML,
            OutputFormat::Mhtml => MonolithOutputFormat::MHTML,
            OutputFormat::Warc => MonolithOutputFormat::WARC,
            OutputFormat::WarcGz => MonolithOutputFormat::WARCGZ,
        }
    }
}

/// Page that got written to disk
struct SavedFile {
    encoding: Option<String>,
    text: String, // For display: same as the URL, except it's not percent-encoded
    url: String,
}

struct MonolithApp {
    target: String,
    output_path: String,
    output_format: OutputFormat,
    output_encoding: Option<&'static str>, // None keeps the document's own encoding
    keep_media: bool,
    keep_fonts: bool,
    keep_frames: bool,
    keep_images: bool,
    keep_scripts: bool,
    keep_styles: bool,
    #[cfg(feature = "browser")]
    browser: bool,
    isolate: bool,
    unwrap_noscript: bool,
    ignore_errors: bool,
    insecure: bool,
    timeout: u64,                     // In seconds
    job: Option<Receiver<JobResult>>, // Some while a page is being saved
    progress: Arc<PrefetchProgress>,  // Of the page being saved
    saved_file: Option<SavedFile>,
    status: String,
}

impl Default for MonolithApp {
    fn default() -> Self {
        Self {
            target: String::new(),
            output_path: default_output_path(),
            output_format: OutputFormat::Html,
            output_encoding: None,
            keep_media: false,
            keep_fonts: false,
            keep_frames: true,
            keep_images: true,
            keep_scripts: true,
            keep_styles: true,
            #[cfg(feature = "browser")]
            browser: false,
            isolate: true,
            unwrap_noscript: false,
            ignore_errors: false,
            insecure: false,
            timeout: TIMEOUT_DEFAULT,
            job: None,
            progress: Arc::default(),
            saved_file: None,
            status: String::new(),
        }
    }
}

// Temporary file of the document being saved, gets deleted if the program exits before that's done
static TEMP_CACHE_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

fn temp_cache_path() -> MutexGuard<'static, Option<PathBuf>> {
    TEMP_CACHE_PATH
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn clean_up() {
    #[cfg(feature = "browser")]
    browser::shut_down();
    if let Some(path) = temp_cache_path().take() {
        let _ = fs::remove_file(path);
    }
}

fn main() -> eframe::Result {
    let _ = ctrlc::set_handler(|| {
        clean_up();
        process::exit(INTERRUPTED_EXIT_CODE);
    });

    let program_name = program_name();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title(program_name.clone())
        .with_inner_size(WINDOW_SIZE)
        .with_resizable(false)
        .with_decorations(false)
        .with_transparent(true); // Needed for rounded window corners
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
        Box::new(|cc| {
            // Stick to the dark theme, otherwise a light system theme would discard these colors
            set_font_sizes(&cc.egui_ctx);
            cc.egui_ctx.set_theme(egui::Theme::Dark);
            cc.egui_ctx
                .options_mut(|options| options.zoom_with_keyboard = false); // The window has a fixed size
            cc.egui_ctx
                .set_visuals_of(egui::Theme::Dark, black_visuals());
            Ok(Box::<MonolithApp>::default())
        }),
    )
}

/// Stock fonts it is, in sizes of our own
fn set_font_sizes(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        for (text_style, size) in [
            (egui::TextStyle::Body, FONT_SIZE),
            (egui::TextStyle::Button, FONT_SIZE),
            (egui::TextStyle::Heading, HEADING_FONT_SIZE),
        ] {
            style
                .text_styles
                .insert(text_style, egui::FontId::proportional(size));
        }
    });
}

fn program_name() -> String {
    let mut program_name: String = env!("CARGO_PKG_NAME").to_string();
    if let Some(l) = program_name.get_mut(0..1) {
        l.make_ascii_uppercase();
    }
    program_name
}

/// White on black color scheme, with solid white buttons and subtle text fields
fn black_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    let border = egui::Stroke::new(1.0, egui::Color32::from_gray(BORDER_GRAY));
    let corner_radius = egui::CornerRadius::same(CORNER_RADIUS);

    visuals.panel_fill = egui::Color32::BLACK;
    visuals.window_fill = egui::Color32::BLACK;
    visuals.window_stroke = border;
    visuals.window_corner_radius = corner_radius;
    visuals.menu_corner_radius = corner_radius;
    visuals.extreme_bg_color = egui::Color32::BLACK;
    visuals.faint_bg_color = egui::Color32::BLACK;
    visuals.code_bg_color = egui::Color32::BLACK;
    visuals.text_edit_bg_color = Some(egui::Color32::from_gray(INPUT_GRAY));
    visuals.text_cursor.stroke.color = egui::Color32::WHITE;
    visuals.hyperlink_color = egui::Color32::WHITE;
    visuals.selection.bg_fill = egui::Color32::from_gray(HIGHLIGHT_GRAY);
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(FOCUS_GRAY));
    visuals.interact_cursor = Some(POINTER_CURSOR); // Covers buttons, but not checkboxes

    // These mostly style buttons, the only stock widget here that relies on all of them;
    // noninteractive also applies to labels and disabled buttons, hence its white text
    let widgets = &mut visuals.widgets;
    for (state, fill, text) in [
        (
            &mut widgets.noninteractive,
            BUTTON_DISABLED_GRAY,
            egui::Color32::WHITE,
        ),
        (&mut widgets.inactive, 0xFF, egui::Color32::BLACK),
        (
            &mut widgets.hovered,
            BUTTON_HOVER_GRAY,
            egui::Color32::BLACK,
        ),
        (
            &mut widgets.active,
            BUTTON_ACTIVE_GRAY,
            egui::Color32::BLACK,
        ),
        (&mut widgets.open, BUTTON_HOVER_GRAY, egui::Color32::BLACK),
    ] {
        state.bg_fill = egui::Color32::from_gray(fill);
        state.weak_bg_fill = egui::Color32::from_gray(fill);
        state.bg_stroke = border;
        state.fg_stroke.color = text;
        state.corner_radius = corner_radius;
        state.expansion = 0.0;
    }

    visuals
}

fn default_output_path() -> String {
    let user_dirs = UserDirs::new();
    match user_dirs.as_ref().and_then(|dirs| dirs.download_dir()) {
        Some(dir) => dir.join(DEFAULT_OUTPUT_FILE_NAME).display().to_string(),
        None => DEFAULT_OUTPUT_FILE_NAME.to_string(),
    }
}

/// Documents can declare their encoding in all kinds of ways, this gets rid of the variety
fn encoding_name(label: &str) -> String {
    Encoding::for_label(label.as_bytes())
        .map_or_else(|| label.to_string(), |encoding| encoding.name().to_string())
}

/// What goes before the path of a saved file
fn saved_message(encoding: Option<&str>) -> String {
    match encoding {
        Some(encoding) => format!("Saved in {encoding} to"),
        None => "Saved to".to_string(),
    }
}

fn saved_file(path: &str, encoding: Option<String>) -> Option<SavedFile> {
    let path = std::path::absolute(path).ok()?;
    let url = Url::from_file_path(&path).ok()?.to_string();
    let display_path = path.display().to_string().replace(MAIN_SEPARATOR, "/");
    let text = format!("file:///{}", display_path.trim_start_matches('/'));
    Some(SavedFile {
        encoding,
        text,
        url,
    })
}

/// Opens the URL with whatever the OS uses for it, without waiting for that to finish
fn open_url(url: &str) {
    let url = url.to_string();
    thread::spawn(move || {
        let mut command = if cfg!(target_os = "windows") {
            let mut command = Command::new("rundll32");
            command.arg("url.dll,FileProtocolHandler");
            command
        } else if cfg!(target_os = "macos") {
            Command::new("open")
        } else {
            Command::new("xdg-open")
        };
        let _ = command.arg(url).status();
    });
}

/// Replaces the middle of the text with an ellipsis if it's wider than max_width
fn elide_middle(ui: &egui::Ui, text: &str, font: &egui::FontId, max_width: f32) -> String {
    let width = |text: &str| {
        ui.painter()
            .layout_no_wrap(text.to_string(), font.clone(), egui::Color32::WHITE)
            .size()
            .x
    };
    if width(text) <= max_width {
        return text.to_string();
    }

    let chars: Vec<char> = text.chars().collect();
    let elided = |kept: usize| -> String {
        let tail = kept / 2;
        let head: String = chars[..kept - tail].iter().collect();
        let tail: String = chars[chars.len() - tail..].iter().collect();
        format!("{head}\u{2026}{tail}")
    };

    // Look for the largest number of characters that still fits
    let (mut low, mut high) = (0, chars.len());
    while low < high {
        let middle = (low + high).div_ceil(2);
        if width(&elided(middle)) <= max_width {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    elided(low)
}

/// Runs on the worker thread: build the document, writing it to disk as it's being made
fn save_document(
    mut session: Session,
    target: String,
    output_path: &str,
    format: OutputFormat,
) -> Result<String, String> {
    // Where it's ended up, once the file has been created
    let saved_path: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    // Directories get files named the default way
    let output_path = output_file_pattern(output_path, DEFAULT_OUTPUT_FILE_NAME);
    let path_holder = Arc::clone(&saved_path);
    let url = parse_target(&target).ok();
    session.set_destination(Box::new(move |title| {
        let path = format_output_path(
            &output_path,
            title.unwrap_or_default(),
            url.as_ref(),
            format.to_monolith(),
        );

        // Directories that aren't there get created too
        let (file, path) = create_output_file(&path)?;
        *path_holder.lock().unwrap() = Some(path);
        Ok(Box::new(BufWriter::new(file)) as Box<dyn Write + Send>)
    }));

    let result = create_monolithic_document(session, target);
    let path = saved_path.lock().unwrap().take();
    match (result, path) {
        (Ok(_), Some(path)) => Ok(path),
        (Ok(_), None) => Err("nothing has been saved".to_string()),
        // The file stays, whatever made it into it may still be of use
        (Err(error), Some(path)) => Err(format!("{error} ({path} is incomplete)")),
        (Err(error), None) => Err(error.to_string()),
    }
}

/// Widths of the label and button columns, shared by the rows that hold a path text field
fn path_row_widths(ui: &egui::Ui) -> (f32, f32) {
    let measure = |text: &str, style: egui::TextStyle| {
        let font = style.resolve(ui.style());
        ui.painter()
            .layout_no_wrap(text.to_string(), font, egui::Color32::WHITE)
            .size()
            .x
            .ceil()
    };
    let label_width = measure(TARGET_LABEL, egui::TextStyle::Body)
        .max(measure(SAVE_TO_LABEL, egui::TextStyle::Body));
    let button_width = measure(TARGET_BUTTON, egui::TextStyle::Button)
        .max(measure(SAVE_TO_BUTTON, egui::TextStyle::Button))
        + 2.0 * ui.spacing().button_padding.x;

    (label_width, button_width)
}

/// Replacement for the native title bar: can be dragged, and it has minimize and close buttons
fn title_bar(ui: &mut egui::Ui) {
    let size = egui::vec2(ui.available_width(), TITLE_BAR_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());

    if response.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        program_name(),
        egui::TextStyle::Body.resolve(ui.style()),
        egui::Color32::WHITE,
    );

    let button_size = egui::Vec2::from(TITLE_BAR_BUTTON_SIZE);
    let close_rect = egui::Align2::RIGHT_CENTER.align_size_within_rect(button_size, rect);
    if title_bar_button(ui, close_rect, "Close", CLOSE_ICON) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
    }

    let minimize_rect = close_rect.translate(egui::vec2(-button_size.x, 0.0));
    if title_bar_button(ui, minimize_rect, "Minimize", MINIMIZE_ICON) {
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }
}

/// Button drawn on top of the title bar; returns true if it got clicked
fn title_bar_button(ui: &egui::Ui, rect: egui::Rect, label: &str, icon: &[[f32; 4]]) -> bool {
    let response = ui
        .interact(rect, ui.id().with(label), egui::Sense::click())
        .on_hover_cursor(POINTER_CURSOR);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));

    let painter = ui.painter();
    let color = if response.hovered() {
        egui::Color32::WHITE
    } else {
        egui::Color32::from_gray(MUTED_GRAY)
    };
    let stroke = egui::Stroke::new(1.5, color);
    // Right-aligned within the button, so that the icon lines up with the rest of the window
    let center = egui::pos2(rect.right() - TITLE_BAR_ICON_EXTENT, rect.center().y);
    for [x1, y1, x2, y2] in icon {
        painter.line_segment(
            [center + egui::vec2(*x1, *y1), center + egui::vec2(*x2, *y2)],
            stroke,
        );
    }

    response.clicked()
}

/// Custom-drawn checkbox: outline when turned off, white square with a black one inside when on
fn checkbox(ui: &mut egui::Ui, checked: &mut bool, label: &str) {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let label_width = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), egui::Color32::WHITE)
        .size()
        .x;
    let size = egui::vec2(
        CHECKBOX_SIZE + CHECKBOX_GAP + label_width,
        CHECKBOX_ROW_HEIGHT,
    );
    let (rect, mut response) = ui.allocate_exact_size(size, egui::Sense::click());
    if response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }
    let is_checked = *checked;
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Checkbox,
            ui.is_enabled(),
            is_checked,
            label,
        )
    });

    // Lights up when turned off, dims the way buttons do when turned on
    let hovered = response.hovered();
    let focused = hovered || response.has_focus();
    let color = match (is_checked, hovered) {
        (true, true) => egui::Color32::from_gray(BUTTON_HOVER_GRAY),
        (true, false) => egui::Color32::WHITE,
        (false, _) if focused => egui::Color32::WHITE,
        (false, _) => egui::Color32::from_gray(MUTED_GRAY),
    };
    let painter = ui.painter();
    let check_box =
        egui::Align2::LEFT_CENTER.align_size_within_rect(egui::Vec2::splat(CHECKBOX_SIZE), rect);
    if is_checked {
        painter.rect_filled(check_box, 3, color);
        painter.rect_filled(
            check_box.shrink(CHECKBOX_MARK_INSET),
            1,
            egui::Color32::BLACK,
        );
    } else {
        let outline = if focused {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_gray(CHECKBOX_OUTLINE_GRAY)
        };
        painter.rect_stroke(
            check_box,
            3,
            egui::Stroke::new(1.0, outline),
            egui::StrokeKind::Inside,
        );
    }

    painter.text(
        check_box.right_center() + egui::vec2(CHECKBOX_GAP, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        font,
        color,
    );

    response.on_hover_cursor(POINTER_CURSOR);
}

/// Button for picking one of the output formats; returns true if it got clicked
fn format_button(ui: &mut egui::Ui, selected: bool, label: &str) -> bool {
    let (_, rect) = ui.allocate_space(egui::vec2(FORMAT_BUTTON_WIDTH, START_BUTTON_HEIGHT));
    let button = if selected {
        egui::Button::new(label) // Stock buttons are the white ones
    } else {
        let text_color = if ui.is_enabled() && ui.rect_contains_pointer(rect) {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_gray(MUTED_GRAY)
        };
        egui::Button::new(egui::RichText::new(label).color(text_color))
            .fill(egui::Color32::from_gray(INPUT_GRAY))
            .stroke(egui::Stroke::new(
                1.0,
                egui::Color32::from_gray(BORDER_GRAY),
            ))
    };
    ui.put(rect, button).clicked()
}

/// Makes the dropdown match text fields, and keeps the items in its popup readable on black
fn dropdown_visuals(visuals: &mut egui::Visuals) {
    let widgets = &mut visuals.widgets;
    for (state, fill) in [
        (&mut widgets.inactive, INPUT_GRAY),
        (&mut widgets.hovered, BORDER_GRAY),
        (&mut widgets.active, BORDER_GRAY),
        (&mut widgets.open, BORDER_GRAY),
    ] {
        state.bg_fill = egui::Color32::from_gray(fill);
        state.weak_bg_fill = egui::Color32::from_gray(fill);
        state.fg_stroke.color = egui::Color32::WHITE;
    }
    visuals.selection.stroke.color = egui::Color32::WHITE;
}

/// Stock buttons reserve no room for the border when not hovered, which makes them change size
fn dropdown_item_visuals(visuals: &mut egui::Visuals) {
    dropdown_visuals(visuals);
    let widgets = &mut visuals.widgets;
    for state in [
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
    ] {
        state.bg_stroke = egui::Stroke::NONE;
    }
}

/// Spinner to the left of the label of a button; drawn on top, the label stays centered
fn save_spinner(ui: &mut egui::Ui, button_rect: egui::Rect, label: &str) {
    let label_width = ui
        .painter()
        .layout_no_wrap(
            label.to_string(),
            egui::TextStyle::Heading.resolve(ui.style()),
            egui::Color32::BLACK,
        )
        .size()
        .x;
    let center = egui::pos2(
        button_rect.center().x - label_width / 2.0 - SPINNER_GAP - SPINNER_SIZE / 2.0,
        button_rect.center().y,
    );
    ui.place(
        egui::Rect::from_center_size(center, egui::Vec2::splat(SPINNER_SIZE)),
        egui::Spinner::new()
            .size(SPINNER_SIZE)
            .color(egui::Color32::BLACK),
    );
}

/// Understands plain seconds, as well as things like "90s", "2m" or "1m 30s"
fn parse_timeout(text: &str) -> Option<f64> {
    let text = text.trim().to_lowercase();
    if text.is_empty() {
        return None;
    }

    let mut seconds = 0.0;
    let mut rest = text.as_str();
    while !rest.is_empty() {
        let number_end = rest
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(rest.len());
        let number: f64 = rest[..number_end].parse().ok()?;
        rest = rest[number_end..].trim_start();

        let unit_end = rest
            .find(|c: char| !c.is_ascii_alphabetic())
            .unwrap_or(rest.len());
        seconds += number
            * match &rest[..unit_end] {
                "" | "s" | "sec" | "secs" | "second" | "seconds" => 1.0,
                "m" | "min" | "mins" | "minute" | "minutes" => 60.0,
                _ => return None,
            };
        rest = rest[unit_end..].trim_start();
    }

    Some(
        seconds
            .round()
            .clamp(TIMEOUT_MIN as f64, TIMEOUT_MAX as f64),
    )
}

/// Label for one of the rows or groups of options, never gets wrapped
fn row_label(ui: &mut egui::Ui, text: &str) {
    let text = egui::RichText::new(text).color(egui::Color32::from_gray(MUTED_GRAY));
    ui.add(egui::Label::new(text).extend());
}

/// Full-width row made of a label, a text field and a button;
/// returns whether the button got clicked, and whether Enter got pressed inside the text field
fn path_row(
    ui: &mut egui::Ui,
    (label, hint, button): (&str, &str, &str),
    (label_width, button_width): (f32, f32),
    value: &mut String,
) -> (bool, bool) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = GRID_SPACING;

        ui.allocate_ui_with_layout(
            egui::vec2(label_width, ROW_HEIGHT),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_width(label_width);
                row_label(ui, label);
            },
        );

        let field_width = (ui.available_width() - button_width - GRID_SPACING).max(MIN_FIELD_WIDTH);
        let field = ui.add_sized(
            [field_width, ROW_HEIGHT],
            egui::TextEdit::singleline(value)
                .hint_text(hint)
                .text_color(egui::Color32::WHITE)
                .margin(INPUT_MARGIN)
                .vertical_align(egui::Align::Center)
                .desired_width(field_width),
        );
        let submitted = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let text_width = ui
            .painter()
            .layout_no_wrap(
                value.clone(),
                egui::TextStyle::Body.resolve(ui.style()),
                egui::Color32::WHITE,
            )
            .size()
            .x;
        if text_width > field_width - INPUT_MARGIN.sum().x {
            field.on_hover_text(value.as_str());
        }

        let clicked = ui
            .add_sized([button_width, ROW_HEIGHT], egui::Button::new(button))
            .clicked();

        (clicked, submitted)
    })
    .inner
}

impl MonolithApp {
    fn start(&mut self, ctx: &egui::Context) {
        let mut options: MonolithOptions = MonolithOptions {
            ignore_errors: self.ignore_errors,
            insecure: self.insecure,
            timeout: self.timeout,
            silent: true,
            ..Default::default()
        };
        if !self.keep_media {
            options.remove_elements = vec!["audio".to_string(), "video".to_string()];
        }
        if !self.keep_frames {
            options.wipe_elements = vec!["frame".to_string(), "iframe".to_string()];
        }
        options.remove_fonts = !self.keep_fonts;
        options.remove_images = !self.keep_images;
        options.remove_styles = !self.keep_styles;
        options.remove_js = !self.keep_scripts;
        #[cfg(feature = "browser")]
        {
            options.browser = self.browser;
        }
        options.isolate = self.isolate;
        options.threads = 30;
        options.unwrap_noscript = self.unwrap_noscript;
        options.output_format = self.output_format.to_monolith();
        options.encoding = self.output_encoding.map(str::to_string);
        let chosen_encoding = self.output_encoding;

        let target = self.target.clone();
        let output_path = self.output_path.clone();
        let format = self.output_format;
        let ctx = ctx.clone();
        let (tx, rx) = mpsc::channel();
        let progress = Arc::new(PrefetchProgress::default());
        self.progress = Arc::clone(&progress);

        thread::spawn(move || {
            // Set up cache (attempt to create temporary file)
            let temp_cache_file: Option<NamedTempFile> =
                Builder::new().prefix("monolith-").tempfile().ok();
            *temp_cache_path() = temp_cache_file
                .as_ref()
                .map(|file| file.path().to_path_buf());
            let cache = Cache::new(
                CACHE_ASSET_FILE_SIZE_THRESHOLD,
                temp_cache_file
                    .as_ref()
                    .map(|file| file.path().display().to_string()),
            );
            let mut session: Session = Session::new(Some(cache), None, options);
            let output_encoding = Arc::new(OnceLock::<String>::new());
            session.output_encoding = Some(Arc::clone(&output_encoding));
            session.prefetch_progress = Some(progress);

            let result = save_document(session, target, &output_path, format).map(|path| {
                let detected_encoding = output_encoding.get().map(|label| encoding_name(label));
                let encoding = chosen_encoding.map(str::to_string).or(detected_encoding);
                (path, encoding)
            });

            // The session (and its cache database) is gone by now,
            // so dropping the temp file removes the database from disk
            drop(temp_cache_file);

            let _ = tx.send(result);
            ctx.request_repaint(); // Wake the UI up to pick up the result
        });

        self.job = Some(rx);
        self.saved_file = None;
        self.status.clear();
    }

    fn poll_job(&mut self) {
        let Some(received) = self.job.as_ref().map(|rx| rx.try_recv()) else {
            return;
        };

        match received {
            Err(TryRecvError::Empty) => return, // Still working
            Ok(Ok((path, encoding))) => {
                self.status = format!("{} {path}", saved_message(encoding.as_deref()));
                self.saved_file = saved_file(&path, encoding);
            }
            Ok(Err(error)) => self.status = format!("Error: {error}"),
            Err(TryRecvError::Disconnected) => {
                self.status = "Error: worker thread stopped unexpectedly".to_string()
            }
        }
        self.job = None;
    }

    fn browse_target(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Pages and archives",
                &[
                    "html",
                    "htm",
                    "mht",
                    "mhtml",
                    "warc",
                    "gz",
                    "har",
                    "webarchive",
                ],
            )
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

    /// Row of options, split into three columns
    fn options_row(&mut self, ui: &mut egui::Ui) {
        ui.scope(|ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend); // Never wrap labels here

            ui.columns_const(|[assets, behavior, network]| {
                assets.spacing_mut().item_spacing.y = CHECKBOX_ROW_SPACING;
                behavior.spacing_mut().item_spacing.y = CHECKBOX_ROW_SPACING;
                network.spacing_mut().item_spacing.y = CHECKBOX_ROW_SPACING;

                checkbox(assets, &mut self.keep_media, "Include audio & video");
                checkbox(assets, &mut self.keep_fonts, "Include fonts");
                checkbox(assets, &mut self.keep_frames, "Include (i)frames");
                checkbox(assets, &mut self.keep_images, "Include images");
                checkbox(assets, &mut self.keep_scripts, "Include scripts");
                checkbox(assets, &mut self.keep_styles, "Include styles");

                checkbox(behavior, &mut self.isolate, "Isolate document");
                checkbox(behavior, &mut self.unwrap_noscript, "Unwrap NOSCRIPT");
                #[cfg(feature = "browser")]
                checkbox(behavior, &mut self.browser, "Render in browser");

                checkbox(network, &mut self.ignore_errors, "Ignore network errors");
                checkbox(network, &mut self.insecure, "Accept invalid certificates");
                network.scope(|ui| self.timeout_slider(ui));
            });
        });
    }

    fn can_start(&self) -> bool {
        self.job.is_none() && !self.target.is_empty() && !self.output_path.is_empty()
    }

    /// Network request timeout, same as the -t flag of the CLI
    fn timeout_slider(&mut self, ui: &mut egui::Ui) {
        row_label(ui, "Network timeout:");

        // Stock colors are meant for buttons: the rail would be white, and so would the value
        dropdown_visuals(ui.visuals_mut());
        ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_gray(BORDER_GRAY);
        ui.spacing_mut().slider_width = ui.available_width() - TIMEOUT_VALUE_WIDTH;

        ui.add(
            egui::Slider::new(&mut self.timeout, TIMEOUT_MIN..=TIMEOUT_SLIDER_MAX)
                .clamping(egui::SliderClamping::Never) // Only the slider itself stops at its maximum
                .custom_parser(parse_timeout)
                .logarithmic(true)
                .trailing_fill(true)
                .suffix("s"),
        )
        .on_hover_cursor(POINTER_CURSOR);
        self.timeout = self.timeout.clamp(TIMEOUT_MIN, TIMEOUT_MAX);
    }

    /// Output format and encoding, all in one row
    fn format_controls(&mut self, ui: &mut egui::Ui) {
        for output_format in OutputFormat::ALL {
            let selected = self.output_format == output_format;
            if format_button(ui, selected, output_format.label()) {
                self.output_format = output_format;
            }
        }
        ui.scope(|ui| self.encoding_dropdown(ui));
    }

    /// Encoding to save the document with, same as the -E flag of the CLI
    fn encoding_dropdown(&mut self, ui: &mut egui::Ui) {
        dropdown_visuals(ui.visuals_mut());

        // Dropdowns grow with their padding, and that also keeps the text vertically centered
        let text_height = ui.text_style_height(&egui::TextStyle::Button);
        ui.spacing_mut().button_padding.y = (START_BUTTON_HEIGHT - text_height) / 2.0;
        ui.spacing_mut().interact_size.y = START_BUTTON_HEIGHT;

        egui::ComboBox::from_id_salt("output_encoding")
            .selected_text(self.output_encoding.unwrap_or(ORIGINAL_ENCODING_LABEL))
            .width(2.0 * FORMAT_BUTTON_WIDTH + FORMAT_COLUMN_SPACING)
            .height(ENCODING_LIST_HEIGHT)
            .show_ui(ui, |ui| {
                dropdown_item_visuals(ui.visuals_mut());
                ui.spacing_mut().button_padding = egui::style::Spacing::default().button_padding;
                ui.selectable_value(&mut self.output_encoding, None, ORIGINAL_ENCODING_LABEL);
                for encoding in OUTPUT_ENCODINGS {
                    ui.selectable_value(&mut self.output_encoding, Some(*encoding), *encoding);
                }
            })
            .response
            .on_hover_cursor(POINTER_CURSOR);
    }

    /// Row with the output format, and the button that starts saving the page
    fn start_row(&mut self, ui: &mut egui::Ui, busy: bool) {
        // Stock rows center everything on a height that grows as they get filled, which
        // puts each of these lower than the previous one and pushes the next row down
        let row_size = egui::vec2(ui.available_width(), START_BUTTON_HEIGHT);
        let row_layout = egui::Layout::left_to_right(egui::Align::Min);
        ui.allocate_ui_with_layout(row_size, row_layout, |ui| {
            ui.spacing_mut().item_spacing.x = FORMAT_COLUMN_SPACING;

            ui.add_enabled_ui(!busy, |ui| self.format_controls(ui));

            let size = [ui.available_width(), START_BUTTON_HEIGHT];
            let label = if busy { "Saving..." } else { "Save" };
            let button = egui::Button::new(egui::RichText::new(label).heading());
            let clicked = ui
                .add_enabled_ui(self.can_start(), |ui| {
                    let response = ui.add_sized(size, button);
                    if busy {
                        save_spinner(ui, response.rect, label);
                    }
                    response.clicked()
                })
                .inner;
            if clicked {
                let ctx = ui.ctx().clone();
                self.start(&ctx);
            }
        });
    }

    /// Bar with the percentage inside of it, which turns black where the bar is filled
    fn progress_bar(&self, ui: &mut egui::Ui) {
        let size = egui::vec2(ui.available_width(), PROGRESS_BAR_HEIGHT);
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(rect, CORNER_RADIUS, egui::Color32::from_gray(INPUT_GRAY));
        painter.rect_stroke(
            rect,
            CORNER_RADIUS,
            egui::Stroke::new(1.0, egui::Color32::from_gray(BORDER_GRAY)),
            egui::StrokeKind::Inside,
        );

        // There's nothing to count until assets get prefetched, which only happens for HTML output
        let (done, total) = self.progress.counts();
        if total == 0 {
            return;
        }
        let percent = done * 100 / total;
        let font = egui::TextStyle::Body.resolve(ui.style());
        let text = format!("{percent}%");
        let filled = egui::Rect::from_min_size(
            rect.min,
            egui::vec2(rect.width() * percent as f32 / 100.0, rect.height()),
        );
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            &text,
            font.clone(),
            egui::Color32::WHITE,
        );
        let filled_painter = ui.painter_at(filled);
        filled_painter.rect_filled(rect, CORNER_RADIUS, egui::Color32::WHITE);
        filled_painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            &text,
            font,
            egui::Color32::BLACK,
        );
    }

    /// Link to the saved file; a tooltip with all of its text is there if it didn't fit
    fn saved_file_message(&self, ui: &mut egui::Ui, saved_file: &SavedFile) {
        ui.horizontal(|ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            ui.spacing_mut().item_spacing.x = GRID_SPACING;

            ui.label(saved_message(saved_file.encoding.as_deref()));

            let font = egui::TextStyle::Body.resolve(ui.style());
            let text = elide_middle(ui, &saved_file.text, &font, ui.available_width());
            let is_elided = text != saved_file.text;
            let mut link = ui.add(egui::Link::new(text));
            if is_elided {
                link = link.on_hover_text(saved_file.text.as_str());
            }
            if link.clicked() {
                open_url(&saved_file.url);
            }
        });
    }

    /// Row for the progress bar while saving, for messages otherwise
    fn status_row(&self, ui: &mut egui::Ui, busy: bool) {
        if busy {
            self.progress_bar(ui);
        } else if let Some(saved_file) = &self.saved_file {
            self.saved_file_message(ui, saved_file);
        } else if !self.status.is_empty() {
            ui.add(egui::Label::new(&self.status).truncate()); // Full text is shown on hover
        }
    }
}

impl eframe::App for MonolithApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        clean_up();
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32() // Shows through the window corners
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_job();
        let busy = self.job.is_some();
        if busy {
            // Worker threads only wake the UI up once they're done
            ui.ctx().request_repaint_after(PROGRESS_REFRESH_INTERVAL);
        }

        let window_frame = egui::Frame::NONE
            .fill(egui::Color32::BLACK)
            .corner_radius(WINDOW_CORNER_RADIUS)
            .inner_margin(WINDOW_MARGIN);

        egui::CentralPanel::default()
            .frame(window_frame)
            .show(ui, |ui| {
                title_bar(ui);

                let widths = path_row_widths(ui);

                ui.add_enabled_ui(!busy, |ui| {
                    let (browse, target_submitted) = path_row(
                        ui,
                        (TARGET_LABEL, "Paste URL or file path", TARGET_BUTTON),
                        widths,
                        &mut self.target,
                    );
                    if browse {
                        self.browse_target();
                    }

                    ui.add_space(ROW_SPACING);
                    self.options_row(ui);

                    ui.add_space(ROW_SPACING);
                    let (browse, save_to_submitted) = path_row(
                        ui,
                        (SAVE_TO_LABEL, "Filesystem path", SAVE_TO_BUTTON),
                        widths,
                        &mut self.output_path,
                    );
                    if browse {
                        self.browse_output_path();
                    }
                    if (target_submitted || save_to_submitted) && self.can_start() {
                        let ctx = ui.ctx().clone();
                        self.start(&ctx);
                    }
                });

                ui.add_space(ROW_SPACING);
                self.start_row(ui, busy);

                ui.add_space(ROW_SPACING);
                self.status_row(ui, busy);
            });
    }
}
