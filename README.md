# CodeSnap

CodeSnap is a lightweight Tauri + Rust + Svelte desktop prototype for creating polished PNG screenshots of code.

## Current prototype

- Svelte UI with an editable CodeMirror code window.
- Popular IDE-style CodeMirror themes: Darcula, GitHub Dark, GitHub Light, Material Dark, and One Dark.
- Solid and gradient backgrounds.
- Adjustable line height.
- CodeMirror language support through `@codemirror/language-data`, including 140+ CodeMirror languages and plain text.
- PNG 2x rendering in the frontend.
- Stable Tauri command stubs for the future tray, global hotkey, clipboard, and native export flow.

## Commands

```bash
npm install
npm run tauri:dev
```

Build the minimal desktop executable without installer bundles:

```bash
npm run dist:minimal
```

The Windows executable is produced at:

```text
src-tauri/target/release/codesnap.exe
```

Installer bundles are intentionally disabled in `src-tauri/tauri.conf.json` for this prototype.
