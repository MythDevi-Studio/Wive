<p align="center">
  <img src="assets/wive-icon.svg" alt="Wive icon" width="88" height="88">
</p>

<h1 align="center">Wive</h1>

<p align="center"><strong>Write code. Stay in flow.</strong></p>

<p align="center">
  A focused, lightweight desktop code editor made with Rust and Skia.
  Designed by Aleverio for Myth Devi Studio (NythDS).
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2024-orange">
  <img alt="Rendering" src="https://img.shields.io/badge/Rendering-Skia-4b9">
  <img alt="License" src="https://img.shields.io/badge/License-Apache--2.0-blue">
</p>

Wive is a calm, dark workspace for opening and editing source files without the weight of a
large IDE. Its custom interface is rendered with Skia, with clear syntax colors, a readable
gutter, and small motion details that keep the editor feeling responsive.

## What you can do

- Open and save files with native file dialogs, or pass a file path when launching Wive.
- Edit text with Unicode-aware cursor movement, selection, and line navigation.
- Select text by dragging or with the keyboard; move and delete by word with standard shortcuts.
- Use undo and redo, find in the current file, and copy, cut, paste, and select all.
- Indent selected lines without replacing them, and preserve indentation and line endings on Enter.
- Work with a project sidebar, line numbers, an active file tab, and a live cursor position.
- Read Rust, JavaScript/TypeScript, Python, JSON, and other text with lightweight syntax colors.
- Keep your changes safe: Wive warns you to save before switching files or starting a new file.
- Save, discard, or cancel before replacing an unsaved document; undo can return to the saved state.
- Render the interface with Skia and adapt it to window resizing and display scaling.
- Check GitHub Releases for updates in the background, then download and verify updates before installing.

## Get started

Prebuilt installers are available from the
[latest GitHub release](https://github.com/MythDevi-Studio/Wive/releases/latest).

Install [Rust](https://www.rust-lang.org/tools/install), then build and launch:

```sh
cargo run --release -p wive-desktop
```

Open a file directly:

```sh
cargo run --release -p wive-desktop -- path/to/file.rs
```

## Keyboard shortcuts

| Action | Shortcut |
| --- | --- |
| Open file | `Ctrl+O` / `Cmd+O` |
| Save | `Ctrl+S` / `Cmd+S` |
| New file | `Ctrl+N` / `Cmd+N` |
| Find | `Ctrl+F` / `Cmd+F` |
| Undo / redo | `Ctrl+Z` / `Ctrl+Y` |
| Select all | `Ctrl+A` / `Cmd+A` |
| Copy / cut / paste | `Ctrl+C` / `Ctrl+X` / `Ctrl+V` |
| Move and select | Arrow keys / `Shift` + arrow keys |
| Move by word / delete by word | `Ctrl+←/→` / `Ctrl+Backspace/Delete` |
| Indent / outdent | `Tab` / `Shift+Tab` |

On macOS, use `Cmd` in place of `Ctrl` for the shortcuts listed above.

## Built with care

Wive is an original project by **Aleverio** and **Myth Devi Studio (NythDS)**. Its interface,
branding, and implementation are created for this project. The codebase uses Rust, Skia,
Winit, and Softbuffer. The interface uses Lucide vector icons; see
[`assets/icons/LICENSE`](assets/icons/LICENSE) for their license notices.

## Project layout

```text
apps/desktop/       Window lifecycle, application events, and startup
crates/foundation/  Shared product identity
crates/text/        Rope-backed Unicode text storage
crates/editor-core/ Editing state, movement, selection, history, and search
crates/syntax/      Lightweight language detection and tokenization
crates/ui/          Skia rendering, layout, and pointer hit testing
crates/platform/    Native file dialogs and clipboard access
crates/updater/     GitHub release checks and checksum-verified updates
assets/             Wive application icon and licensed UI vector icons
packaging/          Windows, macOS, and Linux release packaging metadata
```

Run the checks for all workspace crates with:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Tagged `v*` releases run formatting and lint checks, workspace tests, and then build
Windows, Linux, and universal macOS binaries. GitHub Actions also packages a per-user
Windows setup executable, a macOS disk image containing `Wive.app`, and an x86_64 Debian
package. The release includes the raw platform binaries used by the updater and a
`SHA256SUMS` manifest to verify the release payloads.

On Linux, install the Debian package with `sudo apt install ./wive-linux-x86_64.deb`.
The in-app updater uses that package for Wive installed under `/usr/bin` and requests
authorization through the system's PolicyKit prompt. On Windows, the per-user installer
keeps the executable writable for in-app updates. On macOS, install Wive in a user-writable
Applications folder to allow the updater to replace the app binary; installations requiring
administrator access can use the latest disk image from Releases.

The application version comes from the root `VERSION` file. Changing it on the default
`Dev` branch validates the version, creates a matching `v*` tag, and starts the release
workflow; changing it in a pull request runs CI but does not publish a release.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
