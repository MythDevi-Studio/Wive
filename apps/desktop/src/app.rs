use std::{
    num::NonZeroU32,
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
    time::Instant,
};

use editor_core::Editor;
use foundation::PRODUCT_NAME;
use platform::{clipboard_read, clipboard_write};
use softbuffer::{Context, Surface};
use ui::{
    HoverFrame, HoverTarget, PointerAction, caret_at, handle_pointer, hover_target, render,
    window_icon,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition},
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{Window, WindowAttributes, WindowId},
};

use crate::lifecycle;

enum UpdateMessage {
    Checked(Result<Option<updater::UpdateInfo>, String>),
    InstallFailed(String),
}

pub struct WiveApp {
    editor: Editor,
    window: Option<Arc<Window>>,
    context: Option<Context<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    mouse: Option<(f32, f32)>,
    mouse_selecting: bool,
    modifiers: ModifiersState,
    focused: bool,
    next_cursor_blink: Instant,
    hover_target: HoverTarget,
    hover_transition: Option<(HoverTarget, HoverTarget, Instant)>,
    update: Option<updater::UpdateInfo>,
    installing_update: bool,
    update_tx: Sender<UpdateMessage>,
    update_rx: Receiver<UpdateMessage>,
}

impl WiveApp {
    pub fn new(path: Option<PathBuf>) -> Self {
        let mut editor = Editor::new();
        if let Some(path) = path {
            lifecycle::open_path(&mut editor, path);
        }
        let (update_tx, update_rx) = mpsc::channel();
        let check_tx = update_tx.clone();
        std::thread::spawn(move || {
            let _ = check_tx.send(UpdateMessage::Checked(updater::check_latest()));
        });
        Self {
            editor,
            window: None,
            context: None,
            surface: None,
            mouse: None,
            mouse_selecting: false,
            modifiers: ModifiersState::empty(),
            focused: true,
            next_cursor_blink: Instant::now(),
            hover_target: HoverTarget::None,
            hover_transition: None,
            update: None,
            installing_update: false,
            update_tx,
            update_rx,
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn key_pressed(&mut self, key: &Key, repeat: bool) {
        let command = self.modifiers.control_key() || self.modifiers.super_key();
        let shift = self.modifiers.shift_key();
        if self.modifiers.control_key() && !self.editor.search_open() {
            match key {
                Key::Named(NamedKey::ArrowLeft) => {
                    self.editor.move_word(-1, shift);
                    return;
                }
                Key::Named(NamedKey::ArrowRight) => {
                    self.editor.move_word(1, shift);
                    return;
                }
                Key::Named(NamedKey::Backspace) => {
                    self.editor.delete_word_backward();
                    return;
                }
                Key::Named(NamedKey::Delete) => {
                    self.editor.delete_word_forward();
                    return;
                }
                Key::Named(NamedKey::Home) => {
                    self.editor.move_document_edge(false, shift);
                    return;
                }
                Key::Named(NamedKey::End) => {
                    self.editor.move_document_edge(true, shift);
                    return;
                }
                _ => {}
            }
        } else if self.modifiers.super_key() && !self.editor.search_open() {
            match key {
                Key::Named(NamedKey::ArrowLeft) => {
                    self.editor.move_line_edge(false, shift);
                    return;
                }
                Key::Named(NamedKey::ArrowRight) => {
                    self.editor.move_line_edge(true, shift);
                    return;
                }
                Key::Named(NamedKey::ArrowUp) => {
                    self.editor.move_document_edge(false, shift);
                    return;
                }
                Key::Named(NamedKey::ArrowDown) => {
                    self.editor.move_document_edge(true, shift);
                    return;
                }
                _ => {}
            }
        }
        if command {
            if repeat {
                return;
            }
            let key = match key {
                Key::Character(character) => character.to_lowercase(),
                _ => String::new(),
            };
            match key.as_str() {
                "o" => lifecycle::open_file(&mut self.editor),
                "s" => lifecycle::save_file(&mut self.editor),
                "n" => lifecycle::new_file(&mut self.editor),
                "z" if shift => self.editor.redo(),
                "z" => self.editor.undo(),
                "y" => self.editor.redo(),
                "f" => self.editor.open_search(),
                "a" => self.editor.select_all(),
                "c" => self.copy(false),
                "x" => self.copy(true),
                "v" => self.paste(),
                _ => {}
            }
            return;
        }

        match key {
            Key::Named(NamedKey::ArrowLeft) => self.editor.move_horizontal(-1, shift),
            Key::Named(NamedKey::ArrowRight) => self.editor.move_horizontal(1, shift),
            Key::Named(NamedKey::ArrowUp) => self.editor.move_vertical(-1, shift),
            Key::Named(NamedKey::ArrowDown) => self.editor.move_vertical(1, shift),
            Key::Named(NamedKey::Home) => self.editor.move_line_edge(false, shift),
            Key::Named(NamedKey::End) => self.editor.move_line_edge(true, shift),
            Key::Named(NamedKey::Backspace) if self.editor.search_open() => {
                self.editor.backspace_query()
            }
            Key::Named(NamedKey::Backspace) => self.editor.delete_backward(),
            Key::Named(NamedKey::Delete) => self.editor.delete_forward(),
            Key::Named(NamedKey::Enter) if self.editor.search_open() => self.editor.find_next(),
            Key::Named(NamedKey::Enter) => self.editor.insert_newline(),
            Key::Named(NamedKey::Tab) => self.editor.indent_selection(shift),
            Key::Named(NamedKey::Escape) => self.editor.close_search(),
            Key::Named(NamedKey::F3) => self.editor.find_next(),
            _ => {}
        }
    }

    fn text_input(&mut self, text: &str) {
        if self.modifiers.control_key() || self.modifiers.super_key() {
            return;
        }
        let filtered: String = text
            .chars()
            .filter(|ch| !ch.is_control() && *ch != '\u{7f}')
            .collect();
        if filtered.is_empty() {
            return;
        }
        if self.editor.search_open() {
            self.editor.append_query(&filtered);
        } else {
            self.editor.replace_selection(&filtered);
        }
    }

    fn copy(&mut self, cut: bool) {
        let Some((start, end)) = self.editor.selection() else {
            return;
        };
        let text = self.editor.text().slice(start..end).to_string();
        match clipboard_write(&text) {
            Ok(()) if cut => self.editor.replace_selection(""),
            Ok(()) => {}
            Err(error) => self
                .editor
                .set_status(format!("Clipboard unavailable: {error}")),
        }
    }

    fn paste(&mut self) {
        match clipboard_read() {
            Ok(text) => self.editor.replace_selection(&text),
            Err(error) => self
                .editor
                .set_status(format!("Clipboard unavailable: {error}")),
        }
    }

    fn update_mouse(&mut self, position: PhysicalPosition<f64>) {
        if let Some(window) = &self.window {
            let scale = window.scale_factor();
            let logical_position = ((position.x / scale) as f32, (position.y / scale) as f32);
            self.mouse = Some(logical_position);
            let size = window.inner_size();
            if self.mouse_selecting
                && let Some(position) = caret_at(
                    &self.editor,
                    logical_position.0,
                    logical_position.1,
                    size.width as f32 / scale as f32,
                    size.height as f32 / scale as f32,
                )
            {
                self.editor.set_caret(position, true);
            }
            let target = hover_target(
                &self.editor,
                self.mouse,
                size.width as f32 / scale as f32,
                size.height as f32 / scale as f32,
                self.update.is_some() && !self.installing_update,
            );
            self.set_hover_target(target);
        }
    }

    fn refresh_hover_target(&mut self) {
        if let Some(window) = &self.window {
            let scale = window.scale_factor();
            let size = window.inner_size();
            let target = hover_target(
                &self.editor,
                self.mouse,
                size.width as f32 / scale as f32,
                size.height as f32 / scale as f32,
                self.update.is_some() && !self.installing_update,
            );
            self.set_hover_target(target);
        }
    }

    fn poll_update_messages(&mut self) {
        let mut changed = false;
        while let Ok(message) = self.update_rx.try_recv() {
            match message {
                UpdateMessage::Checked(Ok(Some(update))) => {
                    self.editor
                        .set_status(format!("Wive {} is available to install", update.version));
                    self.update = Some(update);
                    changed = true;
                }
                UpdateMessage::Checked(Ok(None)) => {
                    self.editor.set_status("Wive is up to date");
                    changed = true;
                }
                UpdateMessage::Checked(Err(error)) => {
                    self.editor
                        .set_status(format!("Update check failed: {error}"));
                    changed = true;
                }
                UpdateMessage::InstallFailed(error) => {
                    self.installing_update = false;
                    self.editor
                        .set_status(format!("Could not install update: {error}"));
                    changed = true;
                }
            }
        }
        if changed {
            self.refresh_hover_target();
            self.request_redraw();
        }
    }

    fn install_update(&mut self) {
        if self.editor.is_modified() {
            self.editor
                .set_status("Save your changes before installing an update");
            return;
        }
        let Some(update) = self.update.clone() else {
            return;
        };
        self.installing_update = true;
        self.editor
            .set_status(format!("Installing Wive {}...", update.version));
        self.refresh_hover_target();
        let tx = self.update_tx.clone();
        std::thread::spawn(move || {
            if let Err(error) = updater::install_and_restart(&update) {
                let _ = tx.send(UpdateMessage::InstallFailed(error));
            }
        });
    }

    fn set_hover_target(&mut self, target: HoverTarget) {
        if target != self.hover_target {
            self.hover_transition = Some((self.hover_target, target, Instant::now()));
            self.hover_target = target;
        }
    }

    fn hover_frame(&self) -> HoverFrame {
        let Some((from, to, start)) = self.hover_transition else {
            return HoverFrame {
                from: self.hover_target,
                to: self.hover_target,
                progress: 1.0,
            };
        };
        let progress = (start.elapsed().as_secs_f32() / 0.18).clamp(0.0, 1.0);
        let eased = progress * progress * (3.0 - 2.0 * progress);
        HoverFrame {
            from,
            to,
            progress: eased,
        }
    }
}

impl ApplicationHandler for WiveApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = match event_loop.create_window(
            WindowAttributes::default()
                .with_title(PRODUCT_NAME)
                .with_inner_size(LogicalSize::new(1180.0, 760.0))
                .with_min_inner_size(LogicalSize::new(760.0, 500.0))
                .with_window_icon(Some(window_icon())),
        ) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("{PRODUCT_NAME}: could not create window: {error}");
                event_loop.exit();
                return;
            }
        };
        let context = match Context::new(window.clone()) {
            Ok(context) => context,
            Err(error) => {
                eprintln!("{PRODUCT_NAME}: could not initialize rendering: {error}");
                event_loop.exit();
                return;
            }
        };
        let surface = match Surface::new(&context, window.clone()) {
            Ok(surface) => surface,
            Err(error) => {
                eprintln!("{PRODUCT_NAME}: could not create window surface: {error}");
                event_loop.exit();
                return;
            }
        };
        let size = window.inner_size();
        self.editor
            .set_visible_line_count(visible_lines(size.height, window.scale_factor()));
        self.window = Some(window);
        self.context = Some(context);
        self.surface = Some(surface);
        self.next_cursor_blink = Instant::now() + std::time::Duration::from_millis(450);
        self.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(window) = &self.window else {
            return;
        };
        if window.id() != id {
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                if lifecycle::can_close(&mut self.editor) {
                    event_loop.exit();
                } else {
                    self.request_redraw();
                }
            }
            WindowEvent::Resized(size) => {
                self.editor
                    .set_visible_line_count(visible_lines(size.height, window.scale_factor()));
                if let (Some(width), Some(height)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                    && let Some(surface) = self.surface.as_mut()
                    && let Err(error) = surface.resize(width, height)
                {
                    eprintln!("{PRODUCT_NAME}: could not resize window: {error}");
                }
                self.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => self.request_redraw(),
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                if !focused {
                    self.mouse_selecting = false;
                }
                self.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.update_mouse(position);
                self.request_redraw();
            }
            WindowEvent::CursorLeft { .. } => {
                self.mouse = None;
                self.set_hover_target(HoverTarget::None);
                self.request_redraw();
            }
            WindowEvent::MouseInput { state, button, .. }
                if button == MouseButton::Left && state == ElementState::Released =>
            {
                self.mouse_selecting = false;
            }
            WindowEvent::MouseInput { state, button, .. }
                if button == MouseButton::Left && state == ElementState::Pressed =>
            {
                if let (Some((x, y)), Some(window)) = (self.mouse, self.window.as_ref()) {
                    let size = window.inner_size();
                    let width = size.width as f32 / window.scale_factor() as f32;
                    let height = size.height as f32 / window.scale_factor() as f32;
                    self.mouse_selecting = (self.editor.path().is_some()
                        || !self.editor.is_empty())
                        && caret_at(&self.editor, x, y, width, height).is_some();
                    match handle_pointer(
                        &mut self.editor,
                        x,
                        y,
                        width,
                        height,
                        self.modifiers.shift_key(),
                        self.update.is_some() && !self.installing_update,
                    ) {
                        PointerAction::OpenFile => lifecycle::open_file(&mut self.editor),
                        PointerAction::SaveFile => lifecycle::save_file(&mut self.editor),
                        PointerAction::NewFile => lifecycle::new_file(&mut self.editor),
                        PointerAction::InstallUpdate => self.install_update(),
                        PointerAction::None => {}
                    }
                }

                self.request_redraw();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scale = window.scale_factor();
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as isize,
                    MouseScrollDelta::PixelDelta(position) => {
                        (position.y / (23.0 * scale)) as isize
                    }
                };
                let current = self.editor.scroll_line();
                let next = if lines < 0 {
                    current.saturating_add(lines.unsigned_abs())
                } else {
                    current.saturating_sub(lines as usize)
                };
                self.editor.set_scroll_line(next);
                self.request_redraw();
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                self.key_pressed(&event.logical_key, event.repeat);
                if let Some(text) = &event.text {
                    self.text_input(text);
                }
                self.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let hover = self.hover_frame();
                if let (Some(surface), Some(window)) = (self.surface.as_mut(), self.window.as_ref())
                    && let Err(error) = render(
                        surface,
                        window,
                        &self.editor,
                        self.focused,
                        hover,
                        self.update.is_some(),
                        self.installing_update,
                    )
                {
                    eprintln!("{PRODUCT_NAME}: rendering failed: {error}");
                    event_loop.exit();
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.poll_update_messages();
        let now = Instant::now();
        let animating_hover = self
            .hover_transition
            .is_some_and(|(_, _, start)| now < start + std::time::Duration::from_millis(180));
        if !animating_hover {
            self.hover_transition = None;
        }
        let deadline = if animating_hover {
            now + std::time::Duration::from_millis(16)
        } else {
            self.next_cursor_blink
        };
        event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        if animating_hover {
            self.request_redraw();
        }
        if now >= self.next_cursor_blink {
            self.editor.blink_cursor();
            self.next_cursor_blink = now + std::time::Duration::from_millis(450);
            self.request_redraw();
        }
    }
}

fn visible_lines(height: u32, scale: f64) -> usize {
    ((height as f64 / scale - 114.0) / 23.0).ceil().max(1.0) as usize + 1
}
