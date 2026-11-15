//! Native file dialogs and clipboard access.

use std::path::PathBuf;

/// User choice when closing a document with unsaved edits.
pub enum CloseChoice {
    /// Save, then close if saving succeeds.
    Save,
    /// Discard edits and close.
    Discard,
    /// Keep the window open.
    Cancel,
}

/// Opens a native file picker.
pub fn pick_open_file() -> Option<PathBuf> {
    rfd::FileDialog::new().set_title("Open file").pick_file()
}

/// Opens a native Save As dialog.
pub fn pick_save_file(name: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Save file")
        .set_file_name(name)
        .save_file()
}

/// Copies text to the system clipboard.
pub fn clipboard_write(text: &str) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.set_text(text.to_owned()))
        .map_err(|error| error.to_string())
}

/// Reads text from the system clipboard.
pub fn clipboard_read() -> Result<String, String> {
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.get_text())
        .map_err(|error| error.to_string())
}

/// Asks the user whether to save pending changes before closing the window.
pub fn confirm_close() -> CloseChoice {
    match rfd::MessageDialog::new()
        .set_title("Unsaved changes")
        .set_description("Save your changes before closing Wive?")
        .set_buttons(rfd::MessageButtons::YesNoCancel)
        .show()
    {
        rfd::MessageDialogResult::Yes => CloseChoice::Save,
        rfd::MessageDialogResult::No => CloseChoice::Discard,
        _ => CloseChoice::Cancel,
    }
}
