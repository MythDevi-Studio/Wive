//! Document state and editing commands.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use text::TextBuffer;

/// Editing state for one open document.
pub struct Editor {
    text: TextBuffer,
    path: Option<PathBuf>,
    caret: usize,
    selection_anchor: Option<usize>,
    scroll_line: usize,
    visible_lines: usize,
    modified: bool,
    undo: Vec<TextBuffer>,
    redo: Vec<TextBuffer>,
    status: String,
    query: String,
    search_open: bool,
    match_at: Option<usize>,
    cursor_visible: bool,
}

impl Editor {
    /// Creates an empty untitled document.
    pub fn new() -> Self {
        Self {
            text: TextBuffer::new(),
            path: None,
            caret: 0,
            selection_anchor: None,
            scroll_line: 0,
            visible_lines: 1,
            modified: false,
            undo: Vec::new(),
            redo: Vec::new(),
            status: "Ready".into(),
            query: String::new(),
            search_open: false,
            match_at: None,
            cursor_visible: true,
        }
    }

    /// Returns the current text buffer.
    pub fn text(&self) -> &TextBuffer {
        &self.text
    }

    /// Returns the current file path, if the document has been saved or opened.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Returns the active tab's display name.
    pub fn active_name(&self) -> &str {
        self.path
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled")
    }

    /// Returns the current character offset.
    pub fn caret(&self) -> usize {
        self.caret
    }

    /// Returns the character selection as a half-open range.
    pub fn selection(&self) -> Option<(usize, usize)> {
        self.selection_anchor
            .filter(|&anchor| anchor != self.caret)
            .map(|anchor| (anchor.min(self.caret), anchor.max(self.caret)))
    }

    /// Returns the first visible line.
    pub fn scroll_line(&self) -> usize {
        self.scroll_line
    }

    /// Sets the first visible line, clamped to the document.
    pub fn set_scroll_line(&mut self, line: usize) {
        self.scroll_line = line.min(self.text.len_lines().saturating_sub(self.visible_lines));
    }

    /// Updates the number of document lines visible in the editor viewport.
    pub fn set_visible_line_count(&mut self, lines: usize) {
        self.visible_lines = lines.max(1);
        self.keep_caret_visible();
    }

    /// Returns whether the document contains unsaved changes.
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Returns whether the document has no characters.
    pub fn is_empty(&self) -> bool {
        self.text.char_len() == 0
    }

    /// Returns the latest status message.
    pub fn status(&self) -> &str {
        &self.status
    }

    /// Updates the status message.
    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    /// Returns the current search query.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Returns whether the find bar is open.
    pub fn search_open(&self) -> bool {
        self.search_open
    }

    /// Opens the find bar and clears its query.
    pub fn open_search(&mut self) {
        self.search_open = true;
        self.query.clear();
        self.match_at = None;
        self.status = "Find in file".into();
    }

    /// Closes the find bar.
    pub fn close_search(&mut self) {
        self.search_open = false;
        self.selection_anchor = None;
    }

    /// Returns whether the caret is currently visible.
    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    /// Toggles the caret blink state.
    pub fn blink_cursor(&mut self) {
        self.cursor_visible = !self.cursor_visible;
    }

    /// Loads a UTF-8 file and resets the document history.
    pub fn open(&mut self, path: PathBuf) -> io::Result<()> {
        let contents = fs::read_to_string(&path)?;
        self.text = TextBuffer::from_text(&contents);
        self.path = Some(path);
        self.caret = 0;
        self.selection_anchor = None;
        self.scroll_line = 0;
        self.modified = false;
        self.undo.clear();
        self.redo.clear();
        self.search_open = false;
        self.query.clear();
        self.match_at = None;
        self.status = "File opened".into();
        Ok(())
    }

    /// Saves the document to its current path.
    pub fn save(&mut self) -> io::Result<()> {
        let path = self.path.clone().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "document has no file path")
        })?;
        self.save_to(path)
    }

    /// Saves the document to `path` and sets it as the active file.
    pub fn save_to(&mut self, path: PathBuf) -> io::Result<()> {
        fs::write(&path, self.text.to_string())?;
        self.path = Some(path);
        self.modified = false;
        self.status = "All changes saved".into();
        Ok(())
    }

    /// Starts a new empty document if the current one is clean.
    pub fn new_file(&mut self) -> bool {
        if self.modified {
            self.status = "Save your changes before creating a new file".into();
            return false;
        }
        let visible_lines = self.visible_lines;
        *self = Self::new();
        self.visible_lines = visible_lines;
        self.status = "New file".into();
        true
    }

    /// Places the caret and optionally extends the current selection.
    pub fn set_caret(&mut self, position: usize, extend: bool) {
        if extend {
            self.selection_anchor.get_or_insert(self.caret);
        } else {
            self.selection_anchor = None;
        }
        self.caret = position.min(self.text.char_len());
        self.cursor_visible = true;
    }

    /// Returns the zero-based line and character column at `position`.
    pub fn line_and_column(&self, position: usize) -> (usize, usize) {
        let position = position.min(self.text.char_len());
        let line = self.text.char_to_line(position);
        (line, position - self.text.line_to_char(line))
    }

    /// Replaces the selection, or inserts at the caret, with `insert`.
    pub fn replace_selection(&mut self, insert: &str) {
        let range = self.selection();
        if range.is_none() && insert.is_empty() {
            return;
        }
        self.record_edit();
        let start = range.map_or(self.caret, |selection| selection.0);
        if let Some((_, end)) = range {
            self.text.remove(start..end);
        }
        if !insert.is_empty() {
            self.text.insert(start, insert);
        }
        self.caret = start + insert.chars().count();
        self.selection_anchor = None;
        self.keep_caret_visible();
    }

    /// Moves one character left or right, optionally extending selection.
    pub fn move_horizontal(&mut self, direction: isize, extend: bool) {
        if !extend && let Some((start, end)) = self.selection() {
            self.set_caret(if direction < 0 { start } else { end }, false);
            self.keep_caret_visible();
            return;
        }
        let target = if direction < 0 {
            self.caret.saturating_sub(1)
        } else {
            (self.caret + 1).min(self.text.char_len())
        };
        self.set_caret(target, extend);
        self.keep_caret_visible();
    }

    /// Moves to the start or end of the current line.
    pub fn move_line_edge(&mut self, end: bool, extend: bool) {
        let line = self.text.char_to_line(self.caret);
        let start = self.text.line_to_char(line);
        let target = if end {
            start + line_content_len(&self.text, line)
        } else {
            start
        };
        self.set_caret(target, extend);
        self.keep_caret_visible();
    }

    /// Moves vertically while keeping the current column when possible.
    pub fn move_vertical(&mut self, direction: isize, extend: bool) {
        let (line, column) = self.line_and_column(self.caret);
        let target_line = if direction < 0 {
            line.saturating_sub(1)
        } else {
            (line + 1).min(self.text.len_lines().saturating_sub(1))
        };
        let target = self.text.line_to_char(target_line)
            + column.min(line_content_len(&self.text, target_line));
        self.set_caret(target, extend);
        self.keep_caret_visible();
    }

    /// Deletes the selection or the character immediately before the caret.
    pub fn delete_backward(&mut self) {
        if self.selection().is_some() {
            self.replace_selection("");
        } else if self.caret > 0 {
            self.selection_anchor = Some(self.caret - 1);
            self.replace_selection("");
        }
    }

    /// Deletes the selection or the character immediately after the caret.
    pub fn delete_forward(&mut self) {
        if self.selection().is_some() {
            self.replace_selection("");
        } else if self.caret < self.text.char_len() {
            self.selection_anchor = Some(self.caret + 1);
            self.replace_selection("");
        }
    }

    /// Selects all text in the document.
    pub fn select_all(&mut self) {
        self.selection_anchor = Some(0);
        self.caret = self.text.char_len();
    }

    /// Restores the latest prior edit.
    pub fn undo(&mut self) {
        if let Some(previous) = self.undo.pop() {
            self.redo.push(self.text.clone());
            self.text = previous;
            self.caret = self.caret.min(self.text.char_len());
            self.selection_anchor = None;
            self.modified = true;
            self.status = "Undo".into();
        }
    }

    /// Reapplies the latest undone edit.
    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(self.text.clone());
            self.text = next;
            self.caret = self.caret.min(self.text.char_len());
            self.selection_anchor = None;
            self.modified = true;
            self.status = "Redo".into();
        }
    }

    /// Adds text to the active search query and searches from the current match.
    pub fn append_query(&mut self, text: &str) {
        let start = self.match_at.unwrap_or(self.caret);
        self.query.push_str(text);
        self.find_from(start);
    }

    /// Removes one character from the search query and searches again.
    pub fn backspace_query(&mut self) {
        let start = self.match_at.unwrap_or(self.caret);
        self.query.pop();
        self.find_from(start);
    }

    /// Advances to the next query match, wrapping at the end of the file.
    pub fn find_next(&mut self) {
        let start = self.selection().map_or(self.caret, |(_, end)| end);
        self.find_from(start);
    }

    fn find_from(&mut self, start_position: usize) {
        if self.query.is_empty() {
            self.match_at = None;
            self.selection_anchor = None;
            return;
        }
        let content = self.text.to_string();
        let start = start_position.min(content.chars().count());
        let byte = content
            .char_indices()
            .nth(start)
            .map_or(content.len(), |(byte, _)| byte);
        let found = content[byte..]
            .find(&self.query)
            .map(|offset| byte + offset)
            .or_else(|| content[..byte].find(&self.query));
        if let Some(found) = found {
            let position = content[..found].chars().count();
            self.selection_anchor = Some(position);
            self.caret = position + self.query.chars().count();
            self.match_at = Some(position);
            self.keep_caret_visible();
            self.status = "Match found".into();
        } else {
            self.match_at = None;
            self.selection_anchor = None;
            self.status = "No matches".into();
        }
    }

    fn record_edit(&mut self) {
        self.undo.push(self.text.clone());
        if self.undo.len() > 128 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.modified = true;
        self.cursor_visible = true;
    }

    fn keep_caret_visible(&mut self) {
        let (line, _) = self.line_and_column(self.caret);
        if line < self.scroll_line {
            self.scroll_line = line;
        } else if line >= self.scroll_line + self.visible_lines {
            self.scroll_line = line + 1 - self.visible_lines;
        }
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

fn line_content_len(text: &TextBuffer, line_index: usize) -> usize {
    let line = text.line(line_index);
    let mut length = line.len_chars();
    if length > 0 && line.char(length - 1) == '\n' {
        length -= 1;
    }
    if length > 0 && line.char(length - 1) == '\r' {
        length -= 1;
    }
    length
}

#[cfg(test)]
mod tests {
    use super::Editor;
    use text::TextBuffer;

    #[test]
    fn selection_undo_redo_and_unicode() {
        let mut editor = Editor::new();
        editor.text = TextBuffer::from_text("café!");
        editor.set_caret(4, false);
        editor.set_caret(1, true);
        editor.replace_selection("ó");
        assert_eq!(editor.text().to_string(), "có!");
        editor.undo();
        assert_eq!(editor.text().to_string(), "café!");
        editor.redo();
        assert_eq!(editor.text().to_string(), "có!");
    }

    #[test]
    fn line_end_excludes_crlf() {
        let mut editor = Editor::new();
        editor.text = TextBuffer::from_text("first\r\nsecond");
        editor.move_line_edge(true, false);
        assert_eq!(editor.caret(), 5);
        editor.move_vertical(1, false);
        assert_eq!(editor.line_and_column(editor.caret()), (1, 5));
    }

    #[test]
    fn incremental_find_preserves_match_start() {
        let mut editor = Editor::new();
        editor.text = TextBuffer::from_text("café then café");
        editor.open_search();
        editor.append_query("caf");
        editor.append_query("é");
        assert_eq!(editor.selection(), Some((0, 4)));
        editor.find_next();
        assert_eq!(editor.selection(), Some((10, 14)));
    }

    #[test]
    fn vertical_movement_keeps_the_caret_in_the_viewport() {
        let mut editor = Editor::new();
        editor.text = TextBuffer::from_text(&"row\n".repeat(50));
        editor.set_visible_line_count(10);
        for _ in 0..15 {
            editor.move_vertical(1, false);
        }
        assert_eq!(editor.line_and_column(editor.caret()).0, 15);
        assert_eq!(editor.scroll_line(), 6);
        editor.set_scroll_line(usize::MAX);
        assert_eq!(editor.scroll_line(), 41);
    }

    #[test]
    fn clearing_or_missing_a_search_match_clears_the_old_selection() {
        let mut editor = Editor::new();
        editor.text = TextBuffer::from_text("alpha beta");
        editor.open_search();
        editor.append_query("alpha");
        assert_eq!(editor.selection(), Some((0, 5)));
        editor.append_query("missing");
        assert_eq!(editor.selection(), None);
        for _ in 0..12 {
            editor.backspace_query();
        }
        assert_eq!(editor.selection(), None);
    }
}
