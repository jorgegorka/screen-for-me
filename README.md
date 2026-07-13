# Screen for me

A CleanShot-style screenshot app for macOS and Linux, built with Tauri v2.

- Capture **area / window / fullscreen** from the menu-bar icon or with
  `Cmd/Ctrl+Shift+7 / 8 / 9`
- A quick-access panel appears bottom-left after every capture: **copy,
  save, show in Finder, drag the image straight into other apps**
- Built-in **annotation editor**: arrows, rectangles, ellipses, lines, pen,
  highlighter, text, pixelate, crop — with undo/redo and native-resolution
  export

## Development

```bash
npm install
npm run tauri dev
```

macOS will ask for **Screen Recording** permission on first capture.

## Build

```bash
npm run tauri build
```
