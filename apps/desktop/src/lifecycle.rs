use std::path::PathBuf;

use editor_core::Editor;
use platform::{CloseChoice, confirm_save_changes, pick_open_file, pick_save_file};

pub fn open_file(editor: &mut Editor) {
    if let Some(path) = pick_open_file()
        && resolve_unsaved_changes(editor, "opening another file")
    {
        let result = editor.open(path);
        show_file_result(editor, result);
    }
}

pub fn save_file(editor: &mut Editor) {
    if editor.path().is_some() {
        let result = editor.save();
        show_file_result(editor, result);
    } else {
        save_as(editor);
    }
}

pub fn save_as(editor: &mut Editor) {
    if let Some(path) = pick_save_file(editor.active_name()) {
        let result = editor.save_to(path);
        show_file_result(editor, result);
    }
}

pub fn new_file(editor: &mut Editor) {
    if !editor.is_modified() {
        editor.new_file();
        return;
    }
    match confirm_save_changes("creating a new file") {
        CloseChoice::Save => {
            save_file(editor);
            if !editor.is_modified() {
                editor.new_file();
            }
        }
        CloseChoice::Discard => editor.discard_and_new_file(),
        CloseChoice::Cancel => {}
    }
}

pub fn can_close(editor: &mut Editor) -> bool {
    resolve_unsaved_changes(editor, "closing Wive")
}

fn resolve_unsaved_changes(editor: &mut Editor, action: &str) -> bool {
    if !editor.is_modified() {
        return true;
    }
    match confirm_save_changes(action) {
        CloseChoice::Save => {
            save_file(editor);
            !editor.is_modified()
        }
        CloseChoice::Discard => true,
        CloseChoice::Cancel => false,
    }
}

pub fn open_path(editor: &mut Editor, path: PathBuf) {
    let result = editor.open(path);
    show_file_result(editor, result);
}

fn show_file_result(editor: &mut Editor, result: std::io::Result<()>) {
    if let Err(error) = result {
        editor.set_status(format!("File operation failed: {error}"));
    }
}
