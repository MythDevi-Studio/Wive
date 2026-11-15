//! Skia-rendered desktop editor interface.

use std::sync::Arc;

use editor_core::Editor;
use skia_safe::{
    Canvas, Color, Font, FontMgr, FontStyle, ImageInfo, Paint, PaintStyle, Path, Rect,
};
use softbuffer::Surface;
use syntax::{Language, TokenKind, language_for_file, tokenize_line};
use winit::window::{Icon, Window};

const HEADER_HEIGHT: f32 = 48.0;
const TAB_HEIGHT: f32 = 40.0;
const FOOTER_HEIGHT: f32 = 26.0;
const SIDEBAR_WIDTH: f32 = 236.0;
const GUTTER_WIDTH: f32 = 66.0;
const LINE_HEIGHT: f32 = 23.0;

const BG: Color = Color::from_argb(255, 14, 17, 24);
const PANEL: Color = Color::from_argb(255, 19, 23, 31);
const RAISED: Color = Color::from_argb(255, 26, 31, 41);
const BORDER: Color = Color::from_argb(255, 39, 45, 57);
const TEXT: Color = Color::from_argb(255, 224, 231, 240);
const MUTED: Color = Color::from_argb(255, 139, 151, 170);
const SUBTLE: Color = Color::from_argb(255, 92, 104, 123);
const ACCENT: Color = Color::from_argb(255, 113, 154, 255);
const ACCENT_SOFT: Color = Color::from_argb(255, 35, 48, 73);
const GREEN: Color = Color::from_argb(255, 125, 207, 165);
const YELLOW: Color = Color::from_argb(255, 229, 190, 112);
const PURPLE: Color = Color::from_argb(255, 197, 155, 255);
const CYAN: Color = Color::from_argb(255, 117, 204, 220);
const RED: Color = Color::from_argb(255, 238, 129, 137);

const ICON_ARROW_DOWN: &str = include_str!("../../../assets/icons/arrow-down-to-line.svg");
const ICON_ELLIPSIS: &str = include_str!("../../../assets/icons/ellipsis.svg");
const ICON_FILE_CODE: &str = include_str!("../../../assets/icons/file-code.svg");
const ICON_FILE_PLUS: &str = include_str!("../../../assets/icons/file-plus.svg");
const ICON_FOLDER_OPEN: &str = include_str!("../../../assets/icons/folder-open.svg");
const ICON_SEARCH: &str = include_str!("../../../assets/icons/search.svg");
const ICON_SAVE: &str = include_str!("../../../assets/icons/save.svg");

enum IconShape {
    Path(Path),
    Circle(f32, f32, f32),
}

struct WelcomeCard<'a> {
    icon: &'a str,
    title: &'a str,
    description: &'a str,
    shortcut: &'a str,
}

/// Action requested by a click on the interface.
pub enum PointerAction {
    /// Ask the platform layer to show an Open dialog.
    OpenFile,
    /// Save the current document.
    SaveFile,
    /// Start a new document.
    NewFile,
    /// Begin installing the available update.
    InstallUpdate,
    /// No button action; the click updated the editor caret if relevant.
    None,
}

/// Interactive target currently under the pointer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HoverTarget {
    /// No interactive target.
    #[default]
    None,
    /// Open-file control or welcome card.
    OpenFile,
    /// Save-file control in the application header.
    SaveFile,
    /// New-file welcome card.
    NewFile,
    /// Update control in the application header.
    Update,
}

/// Interpolated state for a short hover transition.
#[derive(Clone, Copy, Debug, Default)]
pub struct HoverFrame {
    /// Target being left.
    pub from: HoverTarget,
    /// Target being entered.
    pub to: HoverTarget,
    /// Eased transition fraction between zero and one.
    pub progress: f32,
}

impl HoverFrame {
    fn amount(self, target: HoverTarget) -> f32 {
        if self.from == self.to {
            return if self.to == target { 1.0 } else { 0.0 };
        }
        if target == self.from {
            1.0 - self.progress
        } else if target == self.to {
            self.progress
        } else {
            0.0
        }
    }
}

/// Renders one complete frame into the native window surface.
pub fn render(
    surface: &mut Surface<Arc<Window>, Arc<Window>>,
    window: &Window,
    editor: &Editor,
    focused: bool,
    hover: HoverFrame,
    update_available: bool,
    installing_update: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let size = window.inner_size();
    if size.width == 0 || size.height == 0 {
        return Ok(());
    }
    let row_bytes = (size.width as usize)
        .checked_mul(4)
        .ok_or("window is too wide")?;
    let mut pixels = vec![
        0_u8;
        row_bytes
            .checked_mul(size.height as usize)
            .ok_or("window is too tall")?
    ];
    let mut raster =
        skia_safe::surfaces::raster_n32_premul((size.width as i32, size.height as i32))
            .ok_or("could not create Skia raster surface")?;
    let scale = window.scale_factor() as f32;
    let canvas = raster.canvas();
    canvas.save();
    canvas.scale((scale, scale));
    draw_ui(
        canvas,
        editor,
        focused,
        hover,
        update_available,
        installing_update,
        (size.width as f32 / scale, size.height as f32 / scale),
    );
    canvas.restore();

    let info = ImageInfo::new_n32_premul((size.width as i32, size.height as i32), None);
    if !raster.read_pixels(&info, &mut pixels, row_bytes, (0, 0)) {
        return Err("Skia could not read the rendered frame".into());
    }
    let mut buffer = surface.buffer_mut()?;
    for (rgba, pixel) in pixels.as_chunks::<4>().0.iter().zip(buffer.iter_mut()) {
        *pixel = (u32::from(rgba[2]) << 16) | (u32::from(rgba[1]) << 8) | u32::from(rgba[0]);
    }
    buffer.present()?;
    Ok(())
}

/// Finds an interactive button or card under the pointer.
pub fn hover_target(
    editor: &Editor,
    mouse: Option<(f32, f32)>,
    width: f32,
    height: f32,
    update_available: bool,
) -> HoverTarget {
    let Some((x, y)) = mouse else {
        return HoverTarget::None;
    };
    if y < HEADER_HEIGHT && x > width - 215.0 && x < width - 131.0 {
        return HoverTarget::OpenFile;
    }
    if y < HEADER_HEIGHT && x > width - 122.0 && x < width - 38.0 {
        return HoverTarget::SaveFile;
    }
    if update_available && y < HEADER_HEIGHT && x > width - 370.0 && x < width - 232.0 {
        return HoverTarget::Update;
    }
    let top = HEADER_HEIGHT + TAB_HEIGHT;
    let bottom = height - FOOTER_HEIGHT;
    if editor.path().is_none() && editor.is_empty() && y >= top {
        let center = (SIDEBAR_WIDTH + width) * 0.5;
        let content_top = top + ((bottom - top) * 0.5 - 152.0).max(45.0);
        let card_y = content_top + 162.0;
        if (card_y..card_y + 78.0).contains(&y) {
            if (center - 198.0..center - 8.0).contains(&x) {
                return HoverTarget::OpenFile;
            }
            if (center + 8.0..center + 198.0).contains(&x) {
                return HoverTarget::NewFile;
            }
        }
    }
    HoverTarget::None
}

/// Maps a pointer position to the closest character in the visible editor text.
pub fn caret_at(editor: &Editor, x: f32, y: f32, width: f32, height: f32) -> Option<usize> {
    let top = HEADER_HEIGHT + TAB_HEIGHT;
    let bottom = height - FOOTER_HEIGHT;
    let text_left = SIDEBAR_WIDTH + GUTTER_WIDTH + 14.0;
    if y < top || y >= bottom || x < text_left || x > width {
        return None;
    }
    if editor.search_open()
        && x >= width - 342.0
        && y >= bottom - 46.0
        && x <= width - 18.0
        && y <= bottom - 10.0
    {
        return None;
    }
    let row = ((y - top - 7.0) / LINE_HEIGHT).max(0.0) as usize;
    let line_index = (editor.scroll_line() + row).min(editor.text().len_lines().saturating_sub(1));
    let column = ((x - text_left) / 8.0).round().max(0.0) as usize;
    Some(editor.text().line_to_char(line_index) + column.min(line_content_len(editor, line_index)))
}

/// Handles a click and updates the caret for clicks inside the editor.
pub fn handle_pointer(
    editor: &mut Editor,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    extend_selection: bool,
    update_available: bool,
) -> PointerAction {
    if y < HEADER_HEIGHT {
        if update_available && x > width - 370.0 && x < width - 232.0 {
            return PointerAction::InstallUpdate;
        }
        if x > width - 122.0 && x < width - 38.0 {
            return PointerAction::SaveFile;
        }
        return if x > width - 215.0 && x < width - 131.0 {
            PointerAction::OpenFile
        } else {
            PointerAction::None
        };
    }
    let top = HEADER_HEIGHT + TAB_HEIGHT;
    let left = SIDEBAR_WIDTH;
    if editor.path().is_none() && editor.is_empty() && y >= top {
        let bottom = height - FOOTER_HEIGHT;
        let center = (left + width) * 0.5;
        let panel_top = top + ((bottom - top) * 0.5 - 152.0).max(45.0);
        let card_y = panel_top + 162.0;
        if (card_y..card_y + 78.0).contains(&y) {
            if (center - 198.0..center - 8.0).contains(&x) {
                return PointerAction::OpenFile;
            }
            if (center + 8.0..center + 198.0).contains(&x) {
                return PointerAction::NewFile;
            }
        }
        return PointerAction::None;
    }
    if let Some(position) = caret_at(editor, x, y, width, height) {
        editor.set_caret(position, extend_selection);
    }
    PointerAction::None
}

/// Builds the small vector-like icon used by the window manager.
pub fn window_icon() -> Icon {
    let side = 64_u32;
    let mut rgba = vec![0; (side * side * 4) as usize];
    for y in 0..side {
        for x in 0..side {
            let dx = x as f32 - 32.0;
            let dy = y as f32 - 32.0;
            let rounded =
                (dx.abs() < 26.0 && dy.abs() < 20.0) || (dy.abs() < 26.0 && dx.abs() < 20.0);
            let inside_w = ((13..=25).contains(&x) && (16..=47).contains(&y))
                || ((25..=48).contains(&x) && (16..=24).contains(&y))
                || ((25..=48).contains(&x) && (28..=35).contains(&y))
                || ((25..=33).contains(&x) && (35..=47).contains(&y));
            let color = if inside_w {
                [14, 17, 24, 255]
            } else if rounded {
                [113, 154, 255, 255]
            } else {
                [0, 0, 0, 0]
            };
            let offset = ((y * side + x) * 4) as usize;
            rgba[offset..offset + 4].copy_from_slice(&color);
        }
    }
    Icon::from_rgba(rgba, side, side).expect("fixed icon dimensions and pixel count")
}

fn draw_ui(
    canvas: &Canvas,
    editor: &Editor,
    focused: bool,
    hover: HoverFrame,
    update_available: bool,
    installing_update: bool,
    size: (f32, f32),
) {
    let (width, height) = size;
    canvas.clear(BG);
    let font = make_font("Inter", 13.0);
    let mono = make_font("monospace", 13.0);
    draw_header(
        canvas,
        &font,
        width,
        hover,
        update_available,
        installing_update,
    );

    let top = HEADER_HEIGHT + TAB_HEIGHT;
    let bottom = height - FOOTER_HEIGHT;
    draw_sidebar(canvas, editor, &font, top, bottom);
    draw_tabs(canvas, editor, &font, width);
    if editor.path().is_some() || !editor.is_empty() {
        draw_editor(canvas, editor, &mono, focused, width, top, bottom);
    } else {
        draw_welcome(canvas, &font, SIDEBAR_WIDTH, top, width, bottom, hover);
    }
    if editor.search_open() {
        draw_search(canvas, editor, &font, width, bottom);
    }
    draw_status(canvas, editor, &font, width, height);
}

fn draw_header(
    canvas: &Canvas,
    font: &Font,
    width: f32,
    hover: HoverFrame,
    update_available: bool,
    installing_update: bool,
) {
    fill(
        canvas,
        Rect::from_xywh(0.0, 0.0, width, HEADER_HEIGHT),
        PANEL,
    );
    rounded(canvas, Rect::from_xywh(17.0, 10.0, 27.0, 27.0), 8.0, ACCENT);
    draw_text(canvas, "w", 23.0, 29.0, &make_font("Inter", 18.0), BG);
    draw_text(canvas, "Wive", 54.0, 28.0, font, TEXT);
    draw_text(canvas, "by Myth Devi Studio", 94.0, 28.0, font, MUTED);
    if update_available || installing_update {
        rounded(
            canvas,
            Rect::from_xywh(width - 366.0, 7.0, 132.0, 34.0),
            8.0,
            if installing_update {
                RAISED
            } else {
                hover_color(hover.amount(HoverTarget::Update))
            },
        );
        draw_text(
            canvas,
            if installing_update {
                "Installing update..."
            } else {
                "Update available"
            },
            width - 337.0,
            29.0,
            &make_font("Inter", 11.0),
            if installing_update { MUTED } else { TEXT },
        );
        draw_icon(canvas, ICON_ARROW_DOWN, width - 357.0, 15.0, 16.0, ACCENT);
    }
    rounded(
        canvas,
        Rect::from_xywh(width - 215.0, 7.0, 84.0, 34.0),
        8.0,
        hover_color(hover.amount(HoverTarget::OpenFile)),
    );
    draw_icon(canvas, ICON_FOLDER_OPEN, width - 205.0, 16.0, 14.0, ACCENT);
    draw_text(canvas, "Open file", width - 185.0, 29.0, font, TEXT);
    rounded(
        canvas,
        Rect::from_xywh(width - 122.0, 7.0, 84.0, 34.0),
        8.0,
        hover_color(hover.amount(HoverTarget::SaveFile)),
    );
    draw_icon(canvas, ICON_SAVE, width - 112.0, 16.0, 14.0, ACCENT);
    draw_text(canvas, "Save", width - 92.0, 29.0, font, TEXT);
    line(
        canvas,
        width - 224.0,
        0.0,
        width - 224.0,
        HEADER_HEIGHT,
        BORDER,
    );
}

fn draw_sidebar(canvas: &Canvas, editor: &Editor, font: &Font, top: f32, bottom: f32) {
    fill(
        canvas,
        Rect::from_xywh(0.0, top, SIDEBAR_WIDTH, bottom - top),
        PANEL,
    );
    line(canvas, SIDEBAR_WIDTH, top, SIDEBAR_WIDTH, bottom, BORDER);
    draw_text(canvas, "EXPLORER", 20.0, top + 27.0, font, MUTED);
    draw_icon(
        canvas,
        ICON_ELLIPSIS,
        SIDEBAR_WIDTH - 42.0,
        top + 14.0,
        16.0,
        MUTED,
    );
    line(canvas, 0.0, top + 43.0, SIDEBAR_WIDTH, top + 43.0, BORDER);
    let project = editor
        .path()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or("MY PROJECT");
    draw_text(
        canvas,
        &project.to_uppercase(),
        17.0,
        top + 69.0,
        font,
        TEXT,
    );
    if editor.path().is_some() {
        rounded(
            canvas,
            Rect::from_xywh(8.0, top + 74.0, SIDEBAR_WIDTH - 16.0, 29.0),
            6.0,
            ACCENT_SOFT,
        );
        draw_icon(
            canvas,
            ICON_FILE_CODE,
            18.0,
            top + 80.0,
            16.0,
            file_color(editor.active_name()),
        );
        draw_text(canvas, editor.active_name(), 40.0, top + 93.0, font, TEXT);
    } else {
        draw_text(canvas, "No folder opened", 17.0, top + 102.0, font, SUBTLE);
        draw_text(
            canvas,
            "Open a file to start editing.",
            17.0,
            top + 126.0,
            font,
            SUBTLE,
        );
    }
}

fn draw_tabs(canvas: &Canvas, editor: &Editor, font: &Font, width: f32) {
    fill(
        canvas,
        Rect::from_xywh(
            SIDEBAR_WIDTH,
            HEADER_HEIGHT,
            width - SIDEBAR_WIDTH,
            TAB_HEIGHT,
        ),
        BG,
    );
    line(
        canvas,
        SIDEBAR_WIDTH,
        HEADER_HEIGHT + TAB_HEIGHT,
        width,
        HEADER_HEIGHT + TAB_HEIGHT,
        BORDER,
    );
    if editor.path().is_some() || !editor.is_empty() || editor.is_modified() {
        fill(
            canvas,
            Rect::from_xywh(SIDEBAR_WIDTH, HEADER_HEIGHT, 176.0, TAB_HEIGHT - 1.0),
            PANEL,
        );
        fill(
            canvas,
            Rect::from_xywh(SIDEBAR_WIDTH, HEADER_HEIGHT, 176.0, 2.0),
            ACCENT,
        );
        draw_icon(
            canvas,
            ICON_FILE_CODE,
            SIDEBAR_WIDTH + 13.0,
            HEADER_HEIGHT + 12.0,
            16.0,
            file_color(editor.active_name()),
        );
        draw_text(
            canvas,
            editor.active_name(),
            SIDEBAR_WIDTH + 35.0,
            HEADER_HEIGHT + 25.0,
            font,
            TEXT,
        );
        circle(
            canvas,
            SIDEBAR_WIDTH + 157.0,
            HEADER_HEIGHT + 20.0,
            3.2,
            if editor.is_modified() { YELLOW } else { MUTED },
        );
    } else {
        draw_text(
            canvas,
            "Welcome",
            SIDEBAR_WIDTH + 20.0,
            HEADER_HEIGHT + 25.0,
            font,
            MUTED,
        );
    }
}

fn draw_editor(
    canvas: &Canvas,
    editor: &Editor,
    font: &Font,
    focused: bool,
    width: f32,
    top: f32,
    bottom: f32,
) {
    let gutter = SIDEBAR_WIDTH + GUTTER_WIDTH;
    let text_left = gutter + 14.0;
    let (active_line, column) = editor.line_and_column(editor.caret());
    let selection = editor.selection();
    let visible_lines = ((bottom - top) / LINE_HEIGHT).ceil() as usize + 1;
    for row in 0..visible_lines {
        let line_index = editor.scroll_line() + row;
        if line_index >= editor.text().len_lines() {
            break;
        }
        let baseline = top + 22.0 + row as f32 * LINE_HEIGHT;
        if line_index == active_line {
            fill(
                canvas,
                Rect::from_xywh(gutter, baseline - 17.0, width - gutter, LINE_HEIGHT),
                Color::from_argb(255, 19, 25, 35),
            );
        }
        if let Some((start, end)) = selection {
            draw_selection(
                canvas,
                editor,
                font,
                SelectionRender {
                    line_index,
                    row,
                    start,
                    end,
                    text_left,
                    top,
                },
            );
        }
        draw_text(
            canvas,
            &format!("{:>4}", line_index + 1),
            SIDEBAR_WIDTH + 10.0,
            baseline,
            font,
            if line_index == active_line {
                ACCENT
            } else {
                SUBTLE
            },
        );
        let line = editor.text().line(line_index).to_string();
        draw_highlighted_line(
            canvas,
            &line,
            text_left,
            baseline,
            font,
            language_for_file(editor.active_name()),
        );
        if focused && editor.cursor_visible() && line_index == active_line && selection.is_none() {
            let before = editor
                .text()
                .line(line_index)
                .slice(..column.min(line_content_len(editor, line_index)))
                .to_string();
            let cursor_x = text_left + font.measure_str(&before, None).0;
            fill(
                canvas,
                Rect::from_xywh(cursor_x, baseline - 16.0, 1.5, 19.0),
                ACCENT,
            );
        }
    }
    let total = editor.text().len_lines();
    if total > visible_lines {
        let rail_height = bottom - top;
        let thumb_height = (rail_height * visible_lines as f32 / total as f32).max(28.0);
        let max_scroll = total.saturating_sub(visible_lines);
        let thumb_y = top
            + (rail_height - thumb_height) * editor.scroll_line().min(max_scroll) as f32
                / max_scroll as f32;
        rounded(
            canvas,
            Rect::from_xywh(width - 10.0, top, 2.0, rail_height),
            1.0,
            BORDER,
        );
        rounded(
            canvas,
            Rect::from_xywh(width - 12.0, thumb_y, 6.0, thumb_height),
            3.0,
            SUBTLE,
        );
    }
}

struct SelectionRender {
    line_index: usize,
    row: usize,
    start: usize,
    end: usize,
    text_left: f32,
    top: f32,
}

fn draw_selection(canvas: &Canvas, editor: &Editor, font: &Font, args: SelectionRender) {
    let SelectionRender {
        line_index,
        row,
        start,
        end,
        text_left,
        top,
    } = args;
    let line_start = editor.text().line_to_char(line_index);
    let line_end = line_start + editor.text().line(line_index).len_chars();
    let start_here = start.max(line_start);
    let end_here = end.min(line_end);
    if start_here >= end_here {
        return;
    }
    let before = editor.text().slice(line_start..start_here).to_string();
    let selected = editor.text().slice(start_here..end_here).to_string();
    let x = text_left + font.measure_str(&before, None).0;
    let width = font.measure_str(&selected, None).0.max(5.0);
    let y = top + 22.0 + row as f32 * LINE_HEIGHT - 17.0;
    fill(
        canvas,
        Rect::from_xywh(x, y, width, LINE_HEIGHT),
        ACCENT_SOFT,
    );
}

fn draw_welcome(
    canvas: &Canvas,
    font: &Font,
    left: f32,
    top: f32,
    width: f32,
    bottom: f32,
    hover: HoverFrame,
) {
    let center = (left + width) * 0.5;
    let content_top = top + ((bottom - top) * 0.5 - 152.0).max(45.0);
    rounded(
        canvas,
        Rect::from_xywh(center - 34.0, content_top, 68.0, 68.0),
        20.0,
        ACCENT_SOFT,
    );
    draw_text(
        canvas,
        "w",
        center - 17.0,
        content_top + 49.0,
        &make_font("Inter", 39.0),
        ACCENT,
    );
    centered_text(
        canvas,
        "A calmer space to create.",
        center,
        content_top + 105.0,
        font,
        TEXT,
    );
    centered_text(
        canvas,
        "A lightweight editor, thoughtfully built by Myth Devi Studio.",
        center,
        content_top + 130.0,
        &make_font("Inter", 12.0),
        MUTED,
    );
    let card_y = content_top + 162.0;
    draw_card(
        canvas,
        WelcomeCard {
            icon: ICON_FOLDER_OPEN,
            title: "Open a file",
            description: "Browse your workspace",
            shortcut: &format!("{} + O", primary_modifier()),
        },
        Rect::from_xywh(center - 198.0, card_y, 190.0, 78.0),
        font,
        hover.amount(HoverTarget::OpenFile),
    );
    draw_card(
        canvas,
        WelcomeCard {
            icon: ICON_FILE_PLUS,
            title: "New file",
            description: "Start with a blank canvas",
            shortcut: &format!("{} + N", primary_modifier()),
        },
        Rect::from_xywh(center + 8.0, card_y, 190.0, 78.0),
        font,
        hover.amount(HoverTarget::NewFile),
    );
}

fn draw_card(canvas: &Canvas, card: WelcomeCard<'_>, rect: Rect, font: &Font, hover: f32) {
    rounded(canvas, rect, 10.0, hover_color(hover));
    stroke_round(canvas, rect, 10.0, BORDER);
    draw_icon(
        canvas,
        card.icon,
        rect.left + 15.0,
        rect.top + 13.0,
        17.0,
        ACCENT,
    );
    draw_text(
        canvas,
        card.title,
        rect.left + 39.0,
        rect.top + 25.0,
        font,
        TEXT,
    );
    let small = make_font("Inter", 11.0);
    draw_text(
        canvas,
        card.description,
        rect.left + 39.0,
        rect.top + 45.0,
        &small,
        MUTED,
    );
    draw_text(
        canvas,
        card.shortcut,
        rect.left + 15.0,
        rect.top + 65.0,
        &small,
        ACCENT,
    );
}

fn draw_search(canvas: &Canvas, editor: &Editor, font: &Font, width: f32, bottom: f32) {
    let rect = Rect::from_xywh(width - 342.0, bottom - 46.0, 324.0, 36.0);
    rounded(canvas, rect, 8.0, RAISED);
    stroke_round(canvas, rect, 8.0, BORDER);
    draw_icon(
        canvas,
        ICON_SEARCH,
        rect.left + 10.0,
        rect.top + 9.0,
        17.0,
        ACCENT,
    );
    let label = if editor.query().is_empty() {
        "Find in file…"
    } else {
        editor.query()
    };
    draw_text(
        canvas,
        label,
        rect.left + 37.0,
        rect.top + 23.0,
        font,
        if editor.query().is_empty() {
            SUBTLE
        } else {
            TEXT
        },
    );
    draw_text(
        canvas,
        "ENTER",
        rect.right - 43.0,
        rect.top + 22.0,
        &make_font("Inter", 9.0),
        MUTED,
    );
}

fn draw_status(canvas: &Canvas, editor: &Editor, font: &Font, width: f32, height: f32) {
    fill(
        canvas,
        Rect::from_xywh(0.0, height - FOOTER_HEIGHT, width, FOOTER_HEIGHT),
        PANEL,
    );
    line(
        canvas,
        0.0,
        height - FOOTER_HEIGHT,
        width,
        height - FOOTER_HEIGHT,
        BORDER,
    );
    circle(
        canvas,
        14.0,
        height - 13.0,
        3.5,
        if editor.is_modified() { YELLOW } else { GREEN },
    );
    draw_text(canvas, editor.status(), 25.0, height - 9.0, font, MUTED);
    let (line, column) = editor.line_and_column(editor.caret());
    draw_text(
        canvas,
        &format!("Ln {}, Col {}", line + 1, column + 1),
        width - 168.0,
        height - 9.0,
        font,
        MUTED,
    );
    draw_text(canvas, "UTF-8", width - 75.0, height - 9.0, font, MUTED);
}

fn draw_highlighted_line(
    canvas: &Canvas,
    line: &str,
    x: f32,
    baseline: f32,
    font: &Font,
    language: Language,
) {
    let mut offset = 0.0;
    for token in tokenize_line(line, language) {
        draw_text(
            canvas,
            token.text,
            x + offset,
            baseline,
            font,
            token_color(token.kind),
        );
        offset += font.measure_str(token.text, None).0;
    }
}

fn token_color(kind: TokenKind) -> Color {
    match kind {
        TokenKind::Keyword => PURPLE,
        TokenKind::String => GREEN,
        TokenKind::Number | TokenKind::Literal => YELLOW,
        TokenKind::Type => CYAN,
        TokenKind::Operator => RED,
        TokenKind::Comment => SUBTLE,
        TokenKind::Plain => TEXT,
    }
}

fn line_content_len(editor: &Editor, line_index: usize) -> usize {
    let line = editor.text().line(line_index);
    let mut length = line.len_chars();
    if length > 0 && line.char(length - 1) == '\n' {
        length -= 1;
    }
    if length > 0 && line.char(length - 1) == '\r' {
        length -= 1;
    }
    length
}

fn file_color(name: &str) -> Color {
    match std::path::Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
    {
        "rs" => YELLOW,
        "md" => CYAN,
        "json" | "toml" | "yaml" | "yml" => YELLOW,
        "js" | "jsx" | "ts" | "tsx" => YELLOW,
        _ => MUTED,
    }
}

fn primary_modifier() -> &'static str {
    if cfg!(target_os = "macos") {
        "⌘"
    } else {
        "Ctrl"
    }
}

fn make_font(family: &str, size: f32) -> Font {
    let typeface = FontMgr::default().match_family_style(family, FontStyle::normal());
    let mut font = typeface
        .map(|typeface| Font::from_typeface(typeface, size))
        .unwrap_or_default();
    font.set_size(size);
    font
}

fn draw_icon(canvas: &Canvas, svg: &str, x: f32, y: f32, size: f32, color: Color) {
    let scale = size / 24.0;
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(2.0);
    paint.set_stroke_cap(skia_safe::paint::Cap::Round);
    paint.set_stroke_join(skia_safe::paint::Join::Round);

    canvas.save();
    canvas.translate((x, y));
    canvas.scale((scale, scale));
    for shape in icon_shapes(svg) {
        match shape {
            IconShape::Path(path) => {
                canvas.draw_path(&path, &paint);
            }
            IconShape::Circle(cx, cy, radius) => {
                canvas.draw_circle((cx, cy), radius, &paint);
            }
        }
    }
    canvas.restore();
}

fn icon_shapes(svg: &str) -> Vec<IconShape> {
    svg.split('<')
        .skip(1)
        .filter_map(|element| {
            let tag = element.split('>').next().unwrap_or_default();
            if tag.starts_with("path ") {
                let data = svg_attribute(tag, "d").expect("bundled icon path has d data");
                Some(IconShape::Path(
                    Path::from_svg(data).expect("bundled icon path is valid SVG"),
                ))
            } else if tag.starts_with("circle ") {
                let coordinate = |name| {
                    svg_attribute(tag, name)
                        .and_then(|value| value.parse::<f32>().ok())
                        .expect("bundled icon circle has valid coordinates")
                };
                Some(IconShape::Circle(
                    coordinate("cx"),
                    coordinate("cy"),
                    coordinate("r"),
                ))
            } else {
                None
            }
        })
        .collect()
}

fn svg_attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("{name}=\"");
    let start = tag.find(&marker)? + marker.len();
    let end = tag[start..].find('"')? + start;
    Some(&tag[start..end])
}

fn draw_text(canvas: &Canvas, text: &str, x: f32, baseline: f32, font: &Font, color: Color) {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.set_anti_alias(true);
    canvas.draw_str(text, (x, baseline), font, &paint);
}

fn centered_text(
    canvas: &Canvas,
    text: &str,
    center: f32,
    baseline: f32,
    font: &Font,
    color: Color,
) {
    let width = font.measure_str(text, None).0;
    draw_text(canvas, text, center - width * 0.5, baseline, font, color);
}

fn fill(canvas: &Canvas, rect: Rect, color: Color) {
    let mut paint = Paint::default();
    paint.set_color(color);
    canvas.draw_rect(rect, &paint);
}

fn rounded(canvas: &Canvas, rect: Rect, radius: f32, color: Color) {
    let mut paint = Paint::default();
    paint.set_color(color);
    canvas.draw_round_rect(rect, radius, radius, &paint);
}

fn stroke_round(canvas: &Canvas, rect: Rect, radius: f32, color: Color) {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.set_style(skia_safe::PaintStyle::Stroke);
    paint.set_stroke_width(1.0);
    canvas.draw_round_rect(rect, radius, radius, &paint);
}

fn line(canvas: &Canvas, x1: f32, y1: f32, x2: f32, y2: f32, color: Color) {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.set_stroke_width(1.0);
    canvas.draw_line((x1, y1), (x2, y2), &paint);
}

fn circle(canvas: &Canvas, x: f32, y: f32, radius: f32, color: Color) {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.set_anti_alias(true);
    canvas.draw_circle((x, y), radius, &paint);
}

fn hover_color(amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    let channel =
        |start: u8, delta: u8| (f32::from(start) + f32::from(delta) * amount).round() as u8;
    Color::from_argb(255, channel(19, 7), channel(23, 8), channel(31, 10))
}

#[cfg(test)]
mod tests {
    use super::{
        HoverFrame, HoverTarget, ICON_ARROW_DOWN, ICON_ELLIPSIS, ICON_FILE_CODE, ICON_FILE_PLUS,
        ICON_FOLDER_OPEN, ICON_SAVE, ICON_SEARCH, PointerAction, caret_at, handle_pointer,
        hover_target, icon_shapes,
    };
    use editor_core::Editor;

    #[test]
    fn hover_interpolates_between_controls() {
        let frame = HoverFrame {
            from: HoverTarget::None,
            to: HoverTarget::OpenFile,
            progress: 0.4,
        };
        assert!((frame.amount(HoverTarget::OpenFile) - 0.4).abs() < f32::EPSILON);
        assert_eq!(frame.amount(HoverTarget::NewFile), 0.0);
    }

    #[test]
    fn welcome_cards_have_distinct_hover_targets() {
        let editor = Editor::new();
        assert_eq!(
            hover_target(&editor, Some((590.0, 478.0)), 1180.0, 760.0, false),
            HoverTarget::OpenFile
        );
        assert_eq!(
            hover_target(&editor, Some((804.0, 478.0)), 1180.0, 760.0, false),
            HoverTarget::NewFile
        );
    }

    #[test]
    fn update_control_is_only_interactive_when_an_update_is_available() {
        let editor = Editor::new();
        let pointer = Some((900.0, 20.0));
        assert_eq!(
            hover_target(&editor, pointer, 1180.0, 760.0, true),
            HoverTarget::Update
        );
        assert_eq!(
            hover_target(&editor, pointer, 1180.0, 760.0, false),
            HoverTarget::None
        );
    }

    #[test]
    fn save_and_open_controls_have_separate_hit_targets() {
        let editor = Editor::new();
        assert_eq!(
            hover_target(&editor, Some((1000.0, 20.0)), 1180.0, 760.0, false),
            HoverTarget::OpenFile
        );
        assert_eq!(
            hover_target(&editor, Some((1100.0, 20.0)), 1180.0, 760.0, false),
            HoverTarget::SaveFile
        );
        let mut editor = Editor::new();
        assert!(matches!(
            handle_pointer(&mut editor, 1100.0, 20.0, 1180.0, 760.0, false, false),
            PointerAction::SaveFile
        ));
    }

    #[test]
    fn bundled_vector_icons_have_parseable_geometry() {
        for svg in [
            ICON_ARROW_DOWN,
            ICON_ELLIPSIS,
            ICON_FILE_CODE,
            ICON_FILE_PLUS,
            ICON_FOLDER_OPEN,
            ICON_SAVE,
            ICON_SEARCH,
        ] {
            assert!(!icon_shapes(svg).is_empty());
        }
    }

    #[test]
    fn pointer_maps_to_visible_text_and_rejects_non_editor_regions() {
        let editor = Editor::new();
        assert_eq!(caret_at(&editor, 316.0, 100.0, 1180.0, 760.0), Some(0));
        assert_eq!(caret_at(&editor, 100.0, 100.0, 1180.0, 760.0), None);
        assert_eq!(caret_at(&editor, 320.0, 20.0, 1180.0, 760.0), None);
    }

    #[test]
    fn search_bar_clicks_do_not_move_the_document_caret() {
        let mut editor = Editor::new();
        editor.open_search();
        assert_eq!(caret_at(&editor, 900.0, 700.0, 1180.0, 760.0), None);
    }
}
