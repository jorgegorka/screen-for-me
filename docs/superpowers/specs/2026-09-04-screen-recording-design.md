# Screen Recording (full display + optional microphone) — v1 plan

## Context

Users keep asking for video. The app is a screenshot utility with a documented "no video capture" v1 scope, so this is a new subsystem, not a tweak. The goal is the simplest useful recording: **record one whole display to an .mp4, optionally mixing in the default microphone so the user can narrate**, then hand the result to the existing quick-access overlay so the capture-to-share loop (drag-out, save, reveal) works for video the way it does for stills.

Decisions already made with the user:

| Question | Decision |
|---|---|
| Mechanism | **Native ScreenCaptureKit in Rust via objc2**, using macOS 15's `SCRecordingOutput` so the framework writes the .mp4 itself (no AVAssetWriter, no sample-buffer plumbing) |
| macOS floor | **macOS 15+ for recording only.** Older systems get an explanatory dialog; screenshots keep working on the app's current floor |
| Platforms | macOS only (tray item and shortcut hidden elsewhere), same as Scrolling Capture |
| Mic choice | A small pre-record glass HUD with a Microphone toggle (remembers last choice) and Record / Cancel |
| Result | Lands in the overlay panel like a still: poster-frame thumbnail, drag-out, Save, Finder, plus Open (system player). No Copy, no Annotate |
| Format | .mp4, H.264, 30 fps, native (Retina) resolution, cursor visible, mic only (no system audio) |

Out of scope for v1: region/window recording, pause, mic device picker, system audio, webcam, trimming, Linux/Windows, GIF.

## Architecture

```
tray "Record Screen" / shortcut ⌘⇧0
  └─ commands::toggle_recording(app)
       ├─ recording in progress → stop_recording()
       ├─ macOS < 15            → dialog recorder.unsupported_*
       └─ else windows::open_recorder(app)   (transient glass HUD, centred on active monitor)
             └─ HUD "Record" → invoke start_recording { microphone }
                  ├─ CAS guard (record_running) ; persist mic choice (recorder_prefs.json)
                  ├─ capture::recording::start(dest.mp4, mic, cursor_point)   [blocking, spawn_blocking]
                  │     SCShareableContent → pick SCDisplay under cursor → SCContentFilter (exclude own app)
                  │     SCStreamConfiguration (px size, 1/30, cursor, captureMicrophone)
                  │     SCRecordingOutput(mp4, h264) → SCStream.startCapture   (wait on completion block)
                  ├─ on Ok: shrink HUD to pill (top-centre of that display), emit record:running, refresh tray
                  └─ watcher thread blocks on recorder events (StopCompleted / StreamStopped / Failed)
                        └─ finish_recording(app): validate mp4 → qlmanage poster → history publish → destroy HUD → refresh tray
stop paths (all call stop_recording → Recorder::request_stop): pill Stop, Esc in pill, shortcut again,
    tray "Stop Recording"; the macOS menu-bar recording indicator arrives as stream:didStopWithError instead.
The recorder's own HUD/overlay windows are kept out of the video by excluding this process in the SCContentFilter
(content_protected is ignored by ScreenCaptureKit on macOS 15+).
```

## Files

### Rust — new

- `src-tauri/src/capture/recording.rs` (`#[cfg(target_os = "macos")]`, registered in `capture/mod.rs` next to `scrolling`)
  - `pub fn supported() -> bool` — `objc2::available!(macos = 15.0)`.
  - `pub struct Recorder` — holds `Retained<SCStream>`, `Retained<SCRecordingOutput>`, the delegate, `dest: PathBuf`, and `events: Receiver<RecorderEvent>`. `unsafe impl Send + Sync` so it can live in `AppState`.
  - `pub enum RecorderEvent { Started, Finished, Failed(String), StreamStopped(String), StopCompleted(Result<(), String>) }` — the stop-completion block sends `StopCompleted` on the same channel so the watcher has a single event source.
  - `pub fn start(dest: &Path, microphone: bool, cursor: (f64, f64)) -> Result<Recorder, CaptureError>` — blocking; each async SCK call is bridged with a `block2::RcBlock` that sends on an `mpsc` channel, waited with a timeout (5 s for shareable content, 10 s for start). Excludes our own process from the filter.
  - `impl Recorder { pub fn request_stop(&self) }` — `stopCaptureWithCompletionHandler` whose block sends `StopCompleted`.
  - `pub fn wait_finished(&self) -> Result<PathBuf, CaptureError>` — blocks on events: `StopCompleted(Ok)` or `StreamStopped` → drain `Finished` for ≤2 s → validate file (`MIN_PLAUSIBLE_MP4_BYTES`, e.g. 16 KiB; below → delete + error); `Failed`/`StopCompleted(Err)` → delete partial file, error. Hard ceiling of 15 s after a stop request before giving up with an error.
  - Delegate: `define_class!` subclass of `NSObject` implementing `SCStreamDelegate` + `SCRecordingOutputDelegate`; ivar holds `Sender<RecorderEvent>`.
  - Pure, unit-tested helpers (no ObjC): `pick_display(displays: &[(u32, Rect)], point) -> Option<u32>` (contains-point, else first), `pill_position(display_bounds) -> (x, y)`, `validate_recording(path)`.
  - Poster: `pub fn write_poster(video: &Path) -> Option<PathBuf>` runs `/usr/bin/qlmanage -t -s 640 -o <dir> <video>`, which writes `<video>.png` beside it. Best effort; `None` on failure.
  - **Exact objc2 identifiers, cargo features and block signatures: see "Native API reference" below (filled from the docs.rs research).**

### Rust — modified

- `src-tauri/Cargo.toml` (macOS block): add `objc2-screen-capture-kit`, `objc2-core-media` (CMTime), `block2`; extend `objc2-foundation` features (`NSProcessInfo`, `NSURL`, `NSError`, `NSString`, `NSArray`). No AVFoundation crate needed. Versions/features per the reference section.
- `src-tauri/tauri.conf.json`: add `bundle.macOS.infoPlist.NSMicrophoneUsageDescription` (and `NSScreenCaptureUsageDescription`) — without the mic key TCC kills the process on first mic access. tauri-build embeds Info.plist into the dev binary too. Do **not** raise `minimumSystemVersion`; the feature is runtime-gated.
- `src-tauri/src/history.rs`
  - `CaptureEntry` gains `kind: CaptureKind` (`Image | Video`, snake_case serde) and `poster: Option<PathBuf>` (video only, present when `<path>.png` exists).
  - `new_capture_path(&self, ext: &str)`; `entry_from_path` accepts `png` and `mp4`. `capture-<ms>.mp4.png` sidecars must be ignored (stem `capture-<ms>.mp4` fails the u64 parse — add a test that asserts it).
  - `prune`: keep 50 entries total and at most `MAX_VIDEOS = 10` videos; removing a video also removes its sidecar poster.
  - Tests: video listed with kind/poster, sidecar ignored, prune removes sidecar and enforces the video cap.
- `src-tauri/src/capture/mod.rs`: `pub mod recording` under macOS cfg. `CaptureMode` unchanged (recording is a toggle, not a mode).
- `src-tauri/src/shortcuts.rs`: `ShortcutAction::Record`, `ACTIONS` → 4 entries, default `CmdOrCtrl+Shift+0`; `mode()` → `Option<CaptureMode>`; `register` dispatches `Record` → `commands::toggle_recording(app)`, others → `trigger_capture`. `defaults_are_valid` covers the new default automatically.
- `src-tauri/src/settings.rs`: `shortcut_record` field + default + `shortcut`/`shortcut_mut` arms (collision resolution in `sanitized` iterates `ACTIONS`, so it picks it up). New `RecorderPrefs { microphone: bool }` + `RecorderPrefsStore = JsonStore<RecorderPrefs>` in `recorder_prefs.json` — separate store so the Settings window's whole-form `set_settings` can't clobber it (same reasoning as `editor_prefs.json`).
- `src-tauri/src/commands.rs`
  - `AppState`: `recorder: Mutex<Option<recording::Recorder>>` (macOS), `record_running: Arc<AtomicBool>`, `recorder_prefs: RecorderPrefsStore`.
  - `pub fn toggle_recording(app)` (tray + shortcut entry), `#[tauri::command] start_recording(app, microphone: bool) -> Result<(), String>`, `#[tauri::command] stop_recording(state) -> Result<(), String>`, `#[tauri::command] recorder_prefs(state) -> RecorderPrefs`, `#[tauri::command] open_capture(app, id)` (opener plugin `open_path`, used by the overlay's Open button), `#[tauri::command] is_recording(state) -> bool`.
  - `start_recording` mirrors `run_scrolling_capture_macos`: requires the `recorder` window, CAS guard on `record_running`, hides the overlay, calls `recording::start` in `spawn_blocking`; on Ok stores the recorder, resizes the window to the pill (`PILL_W 200 × PILL_H 44`, top-centre of the recorded display's logical bounds, 40 px down to clear the menu bar), emits `record:running`, calls `tray::refresh`; on Err returns the message (HUD shows it and rolls back) and drops the guard.
  - Then spawns the watcher: `wait_finished` → `finish_recording(app, path)`: `write_poster`, `publish_capture` (skip `copy_to_clipboard` for `Video`), destroy `recorder` window, `tray::refresh`, `RunningGuard` clears the flag.
  - Error UX: SCK error mentioning permission / `SCStreamErrorUserDeclined` → dialog `perm.screen_recording_*` (same shape as the accessibility dialog); other errors → returned to the HUD.
- `src-tauri/src/tray.rs`: macOS-only item `record_screen`, label `tray.record_screen` or `tray.stop_recording` depending on `record_running`, accelerator from `settings.shortcut(ShortcutAction::Record)`; placed after Scrolling Capture. Handler → `toggle_recording`.
- `src-tauri/src/windows.rs`: `open_recorder(app)` — `transient_window("recorder", "recorder.html", "window.recorder").focused(true)` sized 300×140, centred on `active_monitor` like `open_timer`; destroys an existing instance; returns early if `record_running`. Not in `HIDE_ON_CLOSE` (destroyed on close).
- `src-tauri/src/lib.rs`: register the new commands in `generate_handler!`; manage `RecorderPrefsStore` in `AppState::new`.
- `src-tauri/src/i18n.rs`: no code change; the key-parity test enforces the catalogs.

### Frontend — new

- `recorder.html` (root, like `timer.html`), `src/recorder/main.ts`, `src/recorder/recorder.css`, `src/recorder/format.ts` (+ `format.test.ts`: `formatElapsed(seconds) → "0:07" / "12:34" / "1:02:03"`).
- Armed phase: glass panel (`#18181A` 92%, radius 12, HUD Float shadow) with title `recorder.title`, a Microphone toggle button (`aria-pressed`, violet `#7c5ce6` when on, mic glyph), `Record` (primary) and `Cancel`. Reads initial toggle state from `recorder_prefs`. Esc = close. Start is optimistic like scrollcap: switch to `body.running`, roll back on `invoke` rejection and show the error as the hint.
- Running phase (Rust has shrunk the window): pill with a static red dot `#ee3138`, elapsed time in `font-variant-numeric: tabular-nums` (local `setInterval` started on `record:running`), and `Stop` (primary). Spectrum border on the pill as in `scrollcap.css` `body.running .pill::after`. No pulsing animation (DESIGN.md: no decorative motion).
- `vite.config.ts`: add `recorder: "recorder.html"` to rollup inputs.

### Frontend — modified

- `src/shared/ipc.ts`: `CaptureEntry.kind: "image" | "video"`, `poster?: string`; `Settings.shortcut_record`; `RecorderPrefs`.
- `src/shared/accelerator.ts`: `DEFAULT_ACCELS.record = "CmdOrCtrl+Shift+0"` (drives the Settings shortcuts UI generically).
- `src/shared/dialogs.ts`: generalise `savePngAs` → `saveCaptureAs(defaultPath, kind)` with an mp4 filter (`dialogs.mp4_filter`) for video.
- `src/overlay/main.ts` + `overlay.html` + `overlay.css`: `buildPanel` branches on `entry.kind`. Video: `thumb.src = poster` (fallback: hide img, show a film glyph), add a centred play glyph over the poster, hide `.copy` and `.annotate`, show `.open` (`open_capture`), Save uses the mp4 filter, drag-out uses `icon: entry.poster ?? entry.path`.
- `src/history/main.ts`: poster for video + glyph; omit the Copy button for video.
- `index.html` + `src/settings/main.ts`: Shortcuts tab row for `record` (`shortcut-record`, `-reset`, `-error`), label `settings.shortcut_record`.
- `locales/{en-GB,es,fr,de,it}.json`: `window.recorder`, `tray.record_screen`, `tray.stop_recording`, `recorder.title`, `recorder.microphone`, `recorder.record`, `recorder.cancel`, `recorder.stop`, `recorder.starting`, `recorder.unsupported_title`, `recorder.unsupported_body`, `perm.screen_recording_title`, `perm.screen_recording_body`, `perm.microphone_title`, `perm.microphone_body`, `overlay.open`, `overlay.open_tooltip`, `overlay.latest_recording`, `dialogs.mp4_filter`, `settings.shortcut_record`.

### Docs

- `CLAUDE.md`: drop "No video capture" from scope; add an architecture bullet for recording (SCRecordingOutput, macOS 15 gate, watcher thread, poster sidecar, `recorder_prefs.json`) and gotchas (mic TCC + Info.plist key; the menu-bar recording indicator can stop the stream externally; `content_protected` does not hide windows from ScreenCaptureKit on 15+, so own process is excluded in the filter; keep the `Retained` stream/output/delegate alive or callbacks stop silently; stop-completion, not the finish delegate, is the done signal).
- `DESIGN.md` Spectrum Rule: the "overlay stack badge border" moment no longer exists in code; replace it with "the screen-recording pill border while recording" so the count stays at three. Mention the red recording dot as a semantic, not decorative, colour.
- `PRODUCT.md` scope sentence: add full-screen recording with optional narration.
- Copy this plan's design summary to `docs/superpowers/specs/2026-09-04-screen-recording-design.md` and the task list to `docs/superpowers/plans/2026-09-04-screen-recording.md` (repo convention).

## Native API reference (verified on docs.rs / Apple docs, 2026-09-04)

**Crates** (all compatible with the existing objc2 0.6.4 / block2 0.6.2 / objc2-foundation 0.3 already in `Cargo.lock`):

```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc2-screen-capture-kit = "0.3.2"     # defaults are fine; explicit min set: SCStream, SCShareableContent, SCRecordingOutput, SCError, block2, objc2-core-media, objc2-core-graphics, objc2-core-foundation, std
objc2-core-media = { version = "0.3.2", features = ["CMTime"] }
block2 = "0.6"
objc2-foundation = { version = "0.3", features = ["NSData", "NSProcessInfo", "NSURL", "NSError", "NSString", "NSArray"] }
```
No `objc2-av-foundation`: `SCRecordingOutputConfiguration` defaults to `AVFileTypeMPEG4` + `AVVideoCodecTypeH264`, so only `setOutputURL` is needed. The SCK crate links the framework itself (`#[link(name = "ScreenCaptureKit", kind = "framework")]`).

**Availability gate**: `objc2::available!(macos = 15.0)` before touching any `SCRecordingOutput*` symbol (weak-linked; calling on 14 is UB). `SCStream`/`SCShareableContent` are 12.3+, `captureMicrophone` is 15.0+.

**Start sequence** (all methods are `unsafe fn`; run from `spawn_blocking`, no main thread needed):

1. `SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler(true, true, &block)` with `RcBlock::new(move |content: *mut SCShareableContent, err: *mut NSError| { … tx.send(…) })`; inside the block `Retained::retain(content)` before sending. `rx.recv_timeout(5 s)`. An `NSError` here is the Screen Recording TCC denial → `perm.screen_recording_*` dialog. This call is also what triggers the TCC prompt the first time.
2. Pick the display: iterate `content.displays()`; `SCDisplay::frame()` returns `objc2_core_foundation::CGRect` (convert field-wise; not the `core_graphics` type). `pick_display` picks the frame containing `cursor_point`, else `displays[0]`.
3. Exclude ourselves: find our `SCRunningApplication` in `content.applications()` by `processID() == std::process::id()`; `SCContentFilter::initWithDisplay_excludingApplications_exceptingWindows(alloc, &display, &NSArray::from_retained_slice(&[app]), &NSArray::new())`. **Required**: on macOS 15+ SCK captures the composited framebuffer and ignores `NSWindow.sharingType`/Tauri `content_protected` (tauri #14200), so without this the pill and overlay end up in the video.
4. `SCStreamConfiguration::new()`: `setWidth/Height(points × filter.pointPixelScale())` for native Retina pixels, `setMinimumFrameInterval(CMTime::new(1, 30))`, `setShowsCursor(true)`, `setCapturesAudio(false)`, `setCaptureMicrophone(mic)`, `setMicrophoneCaptureDeviceID(None)`, `setQueueDepth(5)`.
5. Delegate: `define_class!` subclass of `NSObject` (`#[unsafe(super(NSObject))]`, `#[name = "SFMRecordingDelegate"]`, `#[ivars = Ivars { tx: Sender<RecorderEvent> }]`), `unsafe impl NSObjectProtocol`, `unsafe impl SCRecordingOutputDelegate` (`#[unsafe(method(recordingOutputDidStartRecording:))]`, `#[unsafe(method(recordingOutput:didFailWithError:))]`, `#[unsafe(method(recordingOutputDidFinishRecording:))]`) and `unsafe impl SCStreamDelegate` (`#[unsafe(method(stream:didStopWithError:))]`). Bodies only `tx.send(...)` — never panic across FFI. Construct with `Self::alloc().set_ivars(...)` + `msg_send![super(this), init]`; pass as `ProtocolObject::from_ref(&*delegate)`.
6. `SCRecordingOutputConfiguration::new()` + `setOutputURL(&NSURL::fileURLWithPath(&NSString::from_str(dest)))`; `SCRecordingOutput::initWithConfiguration_delegate(alloc, &cfg, delegate)` (delegate is not optional).
7. `SCStream::initWithFilter_configuration_delegate(alloc, &filter, &config, Some(stream_delegate))`; `stream.addRecordingOutput_error(&output)?` **before** start (one output per stream); `startCaptureWithCompletionHandler(Some(&block))` → wait `rx.recv_timeout(10 s)`; an error here with mic on is the Microphone TCC denial.
8. Keep `Retained<SCStream>`, `Retained<SCRecordingOutput>` and the delegate alive in `AppState` for the whole recording (the system holds them weakly; dropping them silently kills callbacks — objc2 #472). They are `!Send`; wrap in `struct Recorder {…}` with `unsafe impl Send + Sync` (SCK is dispatch-queue based; Apple's own sample drives it from actors).

**Stop / finish semantics**:
- `stopCaptureWithCompletionHandler` is the primary "file is finalised" signal ("If stopCapture is called without removing recordingOutput, recording will be stopped and finish writing into the file"). Wait for its completion (10 s timeout), then drain `Finished` from the delegate for up to 2 s as a bonus, then validate the file size. Do **not** block solely on `recordingOutputDidFinishRecording` — whether it fires on a plain `stopCapture` is unverified.
- External stop (the macOS menu-bar recording indicator, display disconnect) arrives as `stream:didStopWithError` → `RecorderEvent::StreamStopped`; the watcher treats it like a stop and validates the file.
- `recordingOutput:didFailWithError:` → `Failed(msg)`: delete the partial file, surface the error.

**Permissions / Info.plist**: SCK is TCC-only (no entitlement). `NSMicrophoneUsageDescription` is **mandatory** for `captureMicrophone` (TCC kills the process without it); `NSScreenCaptureUsageDescription` is undocumented and optional. After the first Screen Recording grant macOS requires an app restart (same gotcha as `screencapture`); macOS 15 also re-prompts periodically. In dev the grant attaches to the responsible process (terminal).

## Implementation order

1. **Spike (throwaway, ~1 h):** an `examples/record_spike.rs` binary in `src-tauri` that records 3 s of the display under the cursor with `captureMicrophone = true` via `SCRecordingOutput`, stops, and exits. Confirms: crate features compile, block bridging and the `define_class!` delegate work, TCC prompts appear, whether `recordingOutputDidFinishRecording` fires after a plain `stopCapture` (decides the 2 s drain), that excluding our process by PID removes a test window from the video, and that `ffprobe` (installed locally) shows an H.264 track at Retina pixel size plus an AAC track. Nothing else proceeds until this plays.
2. `history.rs` kind/extension/poster/prune changes + tests.
3. `recording.rs` module from the spike, with the pure helpers and their tests.
4. Settings/shortcuts/tray/windows/commands wiring + `RecorderPrefs`.
5. Recorder HUD window (html/ts/css/format + test), vite input, locales.
6. Overlay + history + dialogs video branches, Settings shortcut row.
7. tauri.conf.json Info.plist keys; docs updates.
8. `npm run build`, `npm test`, `cd src-tauri && cargo test`.

## Verification

- Automated: `npm run build`, `npm test` (format test, i18n parity, css dark-mode lint), `cargo test` (history, settings, shortcuts, recording helpers).
- Manual on this machine (macOS 26, ffmpeg installed):
  - Tray → Record Screen: HUD appears centred on the cursor's display; toggle mic on; Record. First run prompts Screen Recording and Microphone; grant; retry.
  - Pill appears top-centre, elapsed counter runs, pill is **not** visible in the resulting video, cursor is.
  - Stop via each path: pill Stop, Esc, ⌘⇧0, tray "Stop Recording", and the macOS menu-bar recording indicator. Each yields one overlay card.
  - `ffprobe <captures>/capture-*.mp4` shows h264 at display pixel size, ~30 fps, and an aac track when mic was on, none when off.
  - Overlay: poster thumbnail with play glyph; Open launches QuickTime; Save offers .mp4; Finder reveals; drag-out into Finder/Slack copies the .mp4; Copy/Annotate absent. History window shows the poster.
  - Record with the mic toggle off; reopen HUD; the toggle remembers the last state.
  - Deny Screen Recording in System Settings → dialog explains and no stray window remains.
  - Trigger a screenshot while recording: still works; overlay card for the still appears without disturbing the pill.
  - Prune: with >10 videos the oldest video and its `.mp4.png` sidecar are removed.
