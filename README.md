<p align="center">
  <img src="assets/wive-icon.svg" alt="Wive icon" width="88" height="88">
</p>

<h1 align="center">Wive</h1>

<p align="center"><strong>Write code. Stay in flow.</strong></p>

<p align="center">
  A focused, lightweight desktop code editor made with Rust and Skia.
  Designed by Aleverio for Myth Devi Studio (MythDS).
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
- Use undo and redo, find in the current file, and copy, cut, paste, and select all.
- Work with a project sidebar, line numbers, file tabs, and a live cursor position.
- Read Rust, JavaScript/TypeScript, Python, JSON, and other text with lightweight syntax colors.
- Keep your changes safe: Wive warns you to save before switching files or starting a new file.
- Render the interface with Skia and adapt it to window resizing and display scaling.
- Check GitHub Releases for updates in the background, then download and verify updates before installing.

## Get started

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

On macOS, use `Cmd` in place of `Ctrl` for the shortcuts listed above.

## Built with care

Wive is an original project by **Aleverio** and **Myth Devi Studio (NythDS)**. Its interface,
branding, and implementation are created for this project. The codebase uses Rust, Skia,
Winit, and Softbuffer.

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
assets/             Wive application icon
```

Run the checks for all workspace crates with:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Tagged `v*` releases run formatting and lint checks, workspace tests, and then build
Windows, Linux, and universal macOS binaries. The release workflow publishes the binaries
and their `SHA256SUMS` manifest, which Wive uses to verify in-app updates.
The application version comes from the root `VERSION` file. Changing it on the default
`Dev` branch validates the version, creates a matching `v*` tag, and starts the release
workflow; changing it in a pull request runs CI but does not publish a release.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
