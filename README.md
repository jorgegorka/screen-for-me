# <img src="src-tauri/icons/128x128.png" alt="Screen for me icon" width="32" align="center" /> Screen for me

A fast, polished screenshot app for macOS and Linux, built with Tauri v2.

## Features

- Capture **area / window / fullscreen** from the menu-bar icon or with
  `Cmd/Ctrl+Shift+7 / 8 / 9`
- **Scrolling capture** (macOS): capture an entire scrolling page, stitched
  into one image
- **Timed capture** with an on-screen countdown
- A quick-access panel appears bottom-left after every capture: **copy,
  save, annotate, show in Finder, drag the image straight into other
  apps** — take several captures and they **stack as panels**, newest on
  top, each with its own actions
- **Capture history** window: browse recent captures, copy them, or
  **restore** one back into the quick-access panel
- New captures are **copied to the clipboard** automatically (optional),
  ready to paste
- Built-in **annotation editor**: arrows, rectangles, ellipses, lines, pen,
  highlighter, text, numbered counter steps, pixelate, crop — with undo/redo,
  zoom and native-resolution export
- **Customisable global shortcuts** from the Settings window
- **Localised** into English, Spanish, French, German and Italian (follows
  your system language by default)
- **Launch on start** option backed by the OS login-item state
- **Auto-updates**: checks GitHub Releases and updates in place

## Development

```bash
npm install
npm run tauri dev
```

macOS will ask for **Screen Recording** permission on first capture, and
**Accessibility** permission for scrolling capture.

## Build

```bash
npm run tauri build
```

## Contributors

- [Mario Alvarez](https://github.com/marioalna)
- [Jorge Alvarez](https://github.com/jorgegorka)

## License

Screen for me is available under the [MIT License](https://opensource.org/licenses/MIT).
