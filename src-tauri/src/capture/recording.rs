use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, AnyThread, DefinedClass};
use objc2_core_media::CMTime;
use objc2_foundation::{NSArray, NSError, NSObject, NSObjectProtocol, NSString, NSURL};
use objc2_screen_capture_kit::{
    SCContentFilter, SCDisplay, SCRecordingOutput, SCRecordingOutputConfiguration,
    SCRecordingOutputDelegate, SCShareableContent, SCStream, SCStreamConfiguration,
    SCStreamDelegate,
};

pub const MIN_PLAUSIBLE_MP4_BYTES: u64 = 16 * 1024;
const FRAMES_PER_SECOND: i32 = 30;
const CONTENT_TIMEOUT: Duration = Duration::from_secs(30);
const START_TIMEOUT: Duration = Duration::from_secs(10);
const STOP_TIMEOUT: Duration = Duration::from_secs(10);
const FINISH_GRACE: Duration = Duration::from_secs(5);
const USER_DECLINED_CODE: isize = -3817;

#[derive(Debug, Clone, PartialEq)]
pub enum RecordError {
    ScreenPermission,
    Failed(String),
}

impl std::fmt::Display for RecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScreenPermission => f.write_str("screen recording permission denied"),
            Self::Failed(msg) => f.write_str(msg),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisplayBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl DisplayBounds {
    fn contains(&self, (px, py): (f64, f64)) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }
}

pub fn pick_display(displays: &[DisplayBounds], cursor: Option<(f64, f64)>) -> Option<usize> {
    cursor
        .and_then(|point| displays.iter().position(|d| d.contains(point)))
        .or(if displays.is_empty() { None } else { Some(0) })
}

pub fn pill_position(display: &DisplayBounds, pill_width: f64) -> (f64, f64) {
    const TOP_OFFSET: f64 = 40.0;
    (
        display.x + (display.width - pill_width) / 2.0,
        display.y + TOP_OFFSET,
    )
}

pub fn validate_recording(path: &Path) -> Result<PathBuf, RecordError> {
    match std::fs::metadata(path) {
        Ok(meta) if meta.len() >= MIN_PLAUSIBLE_MP4_BYTES => Ok(path.to_path_buf()),
        Ok(_) => {
            let _ = std::fs::remove_file(path);
            Err(RecordError::Failed("recording produced no usable video".into()))
        }
        Err(_) => Err(RecordError::Failed("recording produced no file".into())),
    }
}

pub fn supported() -> bool {
    objc2::available!(macos = 15.0)
}

#[derive(Debug)]
pub enum RecorderEvent {
    Started,
    Finished,
    Failed(String),
    StreamStopped(String),
    StopCompleted(Result<(), String>),
}

struct Ivars {
    tx: Sender<RecorderEvent>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "SFMRecordingDelegate"]
    #[ivars = Ivars]
    struct RecordingDelegate;

    unsafe impl NSObjectProtocol for RecordingDelegate {}

    unsafe impl SCRecordingOutputDelegate for RecordingDelegate {
        #[unsafe(method(recordingOutputDidStartRecording:))]
        fn did_start(&self, _output: &SCRecordingOutput) {
            let _ = self.ivars().tx.send(RecorderEvent::Started);
        }

        #[unsafe(method(recordingOutput:didFailWithError:))]
        fn did_fail(&self, _output: &SCRecordingOutput, error: &NSError) {
            let _ = self
                .ivars()
                .tx
                .send(RecorderEvent::Failed(error.localizedDescription().to_string()));
        }

        #[unsafe(method(recordingOutputDidFinishRecording:))]
        fn did_finish(&self, _output: &SCRecordingOutput) {
            let _ = self.ivars().tx.send(RecorderEvent::Finished);
        }
    }

    unsafe impl SCStreamDelegate for RecordingDelegate {
        #[unsafe(method(stream:didStopWithError:))]
        fn stream_stopped(&self, _stream: &SCStream, error: &NSError) {
            let _ = self
                .ivars()
                .tx
                .send(RecorderEvent::StreamStopped(error.localizedDescription().to_string()));
        }
    }
);

impl RecordingDelegate {
    fn new(tx: Sender<RecorderEvent>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(Ivars { tx });
        unsafe { msg_send![super(this), init] }
    }
}

pub struct Recorder {
    stream: Retained<SCStream>,
    _output: Retained<SCRecordingOutput>,
    _delegate: Retained<RecordingDelegate>,
    dest: PathBuf,
    display: DisplayBounds,
    events: Receiver<RecorderEvent>,
    tx: Sender<RecorderEvent>,
    stop_requested: AtomicBool,
}

unsafe impl Send for Recorder {}
unsafe impl Sync for Recorder {}

fn error_text(err: *mut NSError) -> Option<String> {
    unsafe { err.as_ref() }.map(|e| e.localizedDescription().to_string())
}

fn classify(err: &NSError) -> RecordError {
    if err.code() == USER_DECLINED_CODE {
        RecordError::ScreenPermission
    } else {
        RecordError::Failed(err.localizedDescription().to_string())
    }
}

fn shareable_content() -> Result<Retained<SCShareableContent>, RecordError> {
    let (tx, rx) = mpsc::channel::<Result<Retained<SCShareableContent>, RecordError>>();
    let block = RcBlock::new(move |content: *mut SCShareableContent, err: *mut NSError| {
        let result = match unsafe { err.as_ref() } {
            Some(e) => Err(classify(e)),
            None => unsafe { Retained::retain(content) }
                .ok_or_else(|| RecordError::Failed("no shareable content".into())),
        };
        let _ = tx.send(result);
    });
    unsafe {
        SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler(
            true, true, &block,
        )
    };
    rx.recv_timeout(CONTENT_TIMEOUT)
        .map_err(|_| RecordError::Failed("could not read screen content in time".into()))?
}

fn bounds_of(display: &SCDisplay) -> DisplayBounds {
    let frame = unsafe { display.frame() };
    DisplayBounds {
        x: frame.origin.x,
        y: frame.origin.y,
        width: frame.size.width,
        height: frame.size.height,
    }
}

pub fn start(dest: &Path, microphone: bool, cursor: Option<(f64, f64)>) -> Result<Recorder, RecordError> {
    let content = shareable_content()?;
    let displays = unsafe { content.displays() };
    let bounds: Vec<DisplayBounds> = displays.iter().map(|d| bounds_of(&d)).collect();
    let index = pick_display(&bounds, cursor)
        .ok_or_else(|| RecordError::Failed("no display available to record".into()))?;
    let display = displays.objectAtIndex(index);
    let display_bounds = bounds[index];

    let pid = std::process::id() as i32;
    let own: Vec<_> = unsafe { content.applications() }
        .iter()
        .filter(|a| unsafe { a.processID() } == pid)
        .collect();
    let filter = unsafe {
        SCContentFilter::initWithDisplay_excludingApplications_exceptingWindows(
            SCContentFilter::alloc(),
            &display,
            &NSArray::from_retained_slice(&own),
            &NSArray::new(),
        )
    };
    let scale = unsafe { filter.pointPixelScale() } as f64;

    let config = unsafe { SCStreamConfiguration::new() };
    unsafe {
        config.setWidth((display_bounds.width * scale).round() as usize);
        config.setHeight((display_bounds.height * scale).round() as usize);
        config.setMinimumFrameInterval(CMTime::new(1, FRAMES_PER_SECOND));
        config.setShowsCursor(true);
        config.setCapturesAudio(false);
        config.setCaptureMicrophone(microphone);
        config.setQueueDepth(5);
    }

    let (tx, events) = mpsc::channel::<RecorderEvent>();
    let delegate = RecordingDelegate::new(tx.clone());
    let output_config = unsafe { SCRecordingOutputConfiguration::new() };
    let url = NSURL::fileURLWithPath(&NSString::from_str(&dest.to_string_lossy()));
    unsafe { output_config.setOutputURL(&url) };
    let output = unsafe {
        SCRecordingOutput::initWithConfiguration_delegate(
            SCRecordingOutput::alloc(),
            &output_config,
            ProtocolObject::from_ref(&*delegate),
        )
    };
    let stream = unsafe {
        SCStream::initWithFilter_configuration_delegate(
            SCStream::alloc(),
            &filter,
            &config,
            Some(ProtocolObject::from_ref(&*delegate)),
        )
    };
    unsafe { stream.addRecordingOutput_error(&output) }
        .map_err(|e| RecordError::Failed(e.localizedDescription().to_string()))?;

    let (start_tx, start_rx) = mpsc::channel::<Result<(), RecordError>>();
    let start_block = RcBlock::new(move |err: *mut NSError| {
        let result = unsafe { err.as_ref() }.map_or(Ok(()), |e| Err(classify(e)));
        let _ = start_tx.send(result);
    });
    unsafe { stream.startCaptureWithCompletionHandler(Some(&start_block)) };
    start_rx
        .recv_timeout(START_TIMEOUT)
        .map_err(|_| RecordError::Failed("recording did not start in time".into()))??;

    Ok(Recorder {
        stream,
        _output: output,
        _delegate: delegate,
        dest: dest.to_path_buf(),
        display: display_bounds,
        events,
        tx,
        stop_requested: AtomicBool::new(false),
    })
}

impl Recorder {
    pub fn display(&self) -> DisplayBounds {
        self.display
    }

    pub fn request_stop(&self) {
        if self.stop_requested.swap(true, Ordering::SeqCst) {
            return;
        }
        let tx = self.tx.clone();
        let block = RcBlock::new(move |err: *mut NSError| {
            let _ = tx.send(RecorderEvent::StopCompleted(error_text(err).map_or(Ok(()), Err)));
        });
        unsafe { self.stream.stopCaptureWithCompletionHandler(Some(&block)) };
    }

    pub fn wait_finished(&self) -> Result<PathBuf, RecordError> {
        let mut stop_deadline: Option<Instant> = None;
        loop {
            let event = match stop_deadline {
                None => self.events.recv().map_err(|_| RecordError::Failed("recorder went away".into()))?,
                Some(deadline) => {
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    match self.events.recv_timeout(remaining) {
                        Ok(event) => event,
                        Err(RecvTimeoutError::Timeout) => break,
                        Err(RecvTimeoutError::Disconnected) => {
                            return Err(RecordError::Failed("recorder went away".into()))
                        }
                    }
                }
            };
            match event {
                RecorderEvent::Started => {
                    if self.stop_requested.load(Ordering::SeqCst) && stop_deadline.is_none() {
                        stop_deadline = Some(Instant::now() + STOP_TIMEOUT);
                    }
                }
                RecorderEvent::Finished => break,
                RecorderEvent::Failed(msg) => {
                    let _ = std::fs::remove_file(&self.dest);
                    return Err(RecordError::Failed(msg));
                }
                RecorderEvent::StopCompleted(Err(msg)) => {
                    let _ = std::fs::remove_file(&self.dest);
                    return Err(RecordError::Failed(msg));
                }
                RecorderEvent::StopCompleted(Ok(())) => {
                    stop_deadline = Some(Instant::now() + FINISH_GRACE);
                }
                RecorderEvent::StreamStopped(reason) => {
                    eprintln!("screen recording stream stopped: {reason}");
                    stop_deadline = Some(Instant::now() + FINISH_GRACE);
                }
            }
        }
        validate_recording(&self.dest)
    }
}

pub fn write_poster(video: &Path) -> Option<PathBuf> {
    let dir = video.parent()?;
    let status = std::process::Command::new("/usr/bin/qlmanage")
        .args(["-t", "-s", "640", "-o"])
        .arg(dir)
        .arg(video)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }
    let poster = crate::history::poster_path(video);
    poster.is_file().then_some(poster)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(x: f64, y: f64, width: f64, height: f64) -> DisplayBounds {
        DisplayBounds { x, y, width, height }
    }

    #[test]
    fn pick_display_prefers_the_one_under_the_cursor() {
        let displays = [display(0.0, 0.0, 1728.0, 1117.0), display(1728.0, -200.0, 2560.0, 1440.0)];
        assert_eq!(pick_display(&displays, Some((2000.0, 100.0))), Some(1));
        assert_eq!(pick_display(&displays, Some((10.0, 10.0))), Some(0));
    }

    #[test]
    fn pick_display_falls_back_to_the_first() {
        let displays = [display(0.0, 0.0, 1728.0, 1117.0), display(1728.0, 0.0, 2560.0, 1440.0)];
        assert_eq!(pick_display(&displays, Some((-50.0, 5000.0))), Some(0));
        assert_eq!(pick_display(&displays, None), Some(0));
        assert_eq!(pick_display(&[], Some((1.0, 1.0))), None);
    }

    #[test]
    fn pill_is_top_centred_on_the_display() {
        let (x, y) = pill_position(&display(1728.0, -200.0, 2560.0, 1440.0), 200.0);
        assert_eq!((x, y), (1728.0 + 1180.0, -160.0));
    }

    #[test]
    fn validate_recording_rejects_missing_and_tiny_files() {
        let dir = std::env::temp_dir().join(format!("sfm-rec-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let missing = dir.join("missing.mp4");
        assert!(validate_recording(&missing).is_err());
        let tiny = dir.join("tiny.mp4");
        std::fs::write(&tiny, b"mp4").unwrap();
        assert!(validate_recording(&tiny).is_err());
        assert!(!tiny.exists());
        let ok = dir.join("ok.mp4");
        std::fs::write(&ok, vec![0u8; MIN_PLAUSIBLE_MP4_BYTES as usize]).unwrap();
        assert_eq!(validate_recording(&ok).unwrap(), ok);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
