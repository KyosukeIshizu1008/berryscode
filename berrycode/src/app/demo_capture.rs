//! Demo capture module — feature showcase with screenshots + video.
//!
//! Uses `bevy::render::view::window::screenshot::Screenshot` to read the
//! actual GPU framebuffer, which works with WGPU/Metal rendering.
//!
//! Activate:  BERRYCODE_DEMO=1 cargo run --bin berrycode
//!
//! Produces per-feature screenshots and a continuous video:
//!   docs/demo/01_startup.png         — Initial startup view
//!   docs/demo/02_explorer.png        — File explorer panel
//!   docs/demo/03_editor.png          — Code editor with syntax highlighting
//!   docs/demo/04_search.png          — Search & replace
//!   docs/demo/05_git.png             — Git integration
//!   docs/demo/06_terminal.png        — Terminal emulator
//!   docs/demo/07_settings.png        — Settings panel
//!   docs/demo/08_ecs_inspector.png   — ECS Inspector
//!   docs/demo/09_bevy_templates.png  — Bevy project templates
//!   docs/demo/10_asset_browser.png   — Asset browser
//!   docs/demo/11_scene_editor.png    — Scene editor / hierarchy
//!   docs/demo/12_debugger.png        — Debugger panel
//!   docs/demo/13_run_panel.png       — Run / build panel
//!   docs/demo/14_tool_panel.png      — Dockable tool panel (Console/Timeline/Dopesheet/Profiler)
//!   docs/demo/demo.mp4               — Continuous video of the full tour

use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::app::types::ActivePanel;

/// Shared state for the ffmpeg video encoder (needs to be Send+Sync for observers).
pub struct VideoEncoder {
    ffmpeg: Option<std::process::Child>,
    frame_count: u64,
    broken: bool,
    width: u32,
    height: u32,
    output_dir: PathBuf,
}

impl VideoEncoder {
    fn new(output_dir: PathBuf) -> Self {
        Self {
            ffmpeg: None,
            frame_count: 0,
            broken: false,
            width: 0,
            height: 0,
            output_dir,
        }
    }

    fn ensure_started(&mut self, w: u32, h: u32) {
        if self.ffmpeg.is_some() || self.broken {
            return;
        }
        self.width = w;
        self.height = h;
        std::fs::create_dir_all(&self.output_dir).ok();
        let out = self.output_dir.join("demo.mp4");
        match std::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "rawvideo",
                "-pixel_format",
                "rgba",
                "-video_size",
                &format!("{}x{}", w, h),
                "-framerate",
                "30",
                "-i",
                "pipe:0",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-preset",
                "fast",
                "-crf",
                "23",
                "-movflags",
                "+faststart",
                out.to_str().unwrap(),
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(c) => {
                tracing::info!("🎬 ffmpeg started: {}x{} → {}", w, h, out.display());
                self.ffmpeg = Some(c);
            }
            Err(e) => {
                tracing::warn!("⚠️ ffmpeg unavailable: {}", e);
                self.broken = true;
            }
        }
    }

    pub fn feed(&mut self, pixels: &[u8], w: u32, h: u32) {
        self.ensure_started(w, h);
        if self.broken {
            return;
        }
        if let Some(ref mut child) = self.ffmpeg {
            if let Some(ref mut stdin) = child.stdin {
                if stdin.write_all(pixels).is_err() {
                    self.broken = true;
                }
                self.frame_count += 1;
            }
        }
    }

    fn finalize(&mut self) {
        if let Some(mut c) = self.ffmpeg.take() {
            drop(c.stdin.take());
            c.wait().ok();
        }
        tracing::info!("🎬 Video: docs/demo/demo.mp4 ({} frames)", self.frame_count);
    }
}

// ── Feature showcase schedule ───────────────────────────────────────────

/// One step in the feature showcase.
struct FeatureStep {
    /// Screenshot filename (e.g. "01_startup.png")
    filename: &'static str,
    /// Which sidebar panel to activate (None = keep current)
    panel: Option<ActivePanel>,
    /// Extra UI state to toggle before the screenshot
    setup: SetupAction,
}

/// Optional UI setup before capturing a feature screenshot.
#[derive(Clone, Copy)]
pub enum SetupAction {
    /// No extra setup needed
    None,
    /// Open the debugger panel
    OpenDebugger,
    /// Open the run / build panel
    OpenRunPanel,
    /// Open the dockable tool panel
    OpenToolPanel,
}

/// Build the ordered list of feature showcase steps.
fn feature_steps() -> Vec<FeatureStep> {
    vec![
        FeatureStep {
            filename: "01_startup.png",
            panel: None,
            setup: SetupAction::None,
        },
        FeatureStep {
            filename: "02_explorer.png",
            panel: Some(ActivePanel::Explorer),
            setup: SetupAction::None,
        },
        FeatureStep {
            filename: "03_editor.png",
            panel: Some(ActivePanel::Explorer),
            setup: SetupAction::None,
        },
        FeatureStep {
            filename: "04_search.png",
            panel: Some(ActivePanel::Search),
            setup: SetupAction::None,
        },
        FeatureStep {
            filename: "05_git.png",
            panel: Some(ActivePanel::Git),
            setup: SetupAction::None,
        },
        FeatureStep {
            filename: "06_terminal.png",
            panel: Some(ActivePanel::Terminal),
            setup: SetupAction::None,
        },
        FeatureStep {
            filename: "07_settings.png",
            panel: Some(ActivePanel::Settings),
            setup: SetupAction::None,
        },
        FeatureStep {
            filename: "08_ecs_inspector.png",
            panel: Some(ActivePanel::EcsInspector),
            setup: SetupAction::None,
        },
        FeatureStep {
            filename: "09_bevy_templates.png",
            panel: Some(ActivePanel::BevyTemplates),
            setup: SetupAction::None,
        },
        FeatureStep {
            filename: "10_asset_browser.png",
            panel: Some(ActivePanel::AssetBrowser),
            setup: SetupAction::None,
        },
        FeatureStep {
            filename: "11_scene_editor.png",
            panel: Some(ActivePanel::SceneEditor),
            setup: SetupAction::None,
        },
        FeatureStep {
            filename: "12_debugger.png",
            panel: Some(ActivePanel::Explorer),
            setup: SetupAction::OpenDebugger,
        },
        FeatureStep {
            filename: "13_run_panel.png",
            panel: Some(ActivePanel::Explorer),
            setup: SetupAction::OpenRunPanel,
        },
        FeatureStep {
            filename: "14_tool_panel.png",
            panel: Some(ActivePanel::Explorer),
            setup: SetupAction::OpenToolPanel,
        },
    ]
}

// ── Demo state machine ──────────────────────────────────────────────────

/// Demo capture state — stored in BerryCodeApp.
pub struct DemoCapture {
    pub active: bool,
    start_time: Option<Instant>,
    output_dir: PathBuf,
    pub screenshots: Vec<String>,
    pub encoder: Arc<Mutex<VideoEncoder>>,
    /// Index into the feature_steps list
    step_index: usize,
    /// Whether the current step's UI has been set up (wait 1 frame before capture)
    step_setup_done: bool,
    /// How many frames we've waited after setup (allow UI to settle)
    settle_frames: u32,
    /// Whether we're in the trailing video recording phase after all screenshots
    trailing_video: bool,
    done: bool,
}

/// Wait time before the first screenshot (let UI fully initialize)
const INITIAL_WAIT_SECS: f32 = 3.0;
/// Frames to wait after switching UI state before taking screenshot
const SETTLE_FRAMES: u32 = 15;
/// Seconds of extra video to record after the last screenshot
const TRAILING_VIDEO_SECS: f32 = 3.0;

impl DemoCapture {
    pub fn new() -> Self {
        let active = std::env::var("BERRYCODE_DEMO").is_ok();
        let output_dir = PathBuf::from("docs/demo");
        if active {
            eprintln!("[DemoCapture] BERRYCODE_DEMO detected — feature showcase enabled");
            eprintln!(
                "[DemoCapture] Will capture {} feature screenshots + video",
                feature_steps().len()
            );
        }
        Self {
            active,
            start_time: None,
            output_dir: output_dir.clone(),
            screenshots: Vec::new(),
            encoder: Arc::new(Mutex::new(VideoEncoder::new(output_dir))),
            step_index: 0,
            step_setup_done: false,
            settle_frames: 0,
            trailing_video: false,
            done: false,
        }
    }

    /// Call every frame. Returns an action for the Bevy system to execute.
    pub fn tick(&mut self) -> DemoAction {
        if !self.active || self.done {
            return DemoAction::None;
        }

        // Initialize on first tick
        if self.start_time.is_none() {
            self.start_time = Some(Instant::now());
            std::fs::create_dir_all(&self.output_dir).ok();
            tracing::info!(
                "🎬 Feature showcase started → {}",
                self.output_dir.display()
            );
        }

        let elapsed = self.start_time.unwrap().elapsed().as_secs_f32();

        // Wait for UI to initialize before starting
        if elapsed < INITIAL_WAIT_SECS {
            return DemoAction::CaptureVideo;
        }

        let steps = feature_steps();

        // Trailing video phase (after all screenshots are done)
        if self.trailing_video {
            let last_shot_time =
                INITIAL_WAIT_SECS + (steps.len() as f32) * (SETTLE_FRAMES as f32 / 30.0 + 0.1);
            if elapsed >= last_shot_time + TRAILING_VIDEO_SECS {
                return DemoAction::Finish;
            }
            return DemoAction::CaptureVideo;
        }

        // All steps done → enter trailing video phase
        if self.step_index >= steps.len() {
            self.trailing_video = true;
            return DemoAction::CaptureVideo;
        }

        let step = &steps[self.step_index];

        // Set up UI state for this step
        if !self.step_setup_done {
            self.step_setup_done = true;
            self.settle_frames = 0;
            return DemoAction::SetupUi {
                panel: step.panel,
                setup: step.setup,
            };
        }

        // Wait for UI to settle
        self.settle_frames += 1;
        if self.settle_frames < SETTLE_FRAMES {
            return DemoAction::CaptureVideo;
        }

        // Capture screenshot + video frame
        let filename = step.filename.to_string();
        self.step_index += 1;
        self.step_setup_done = false;
        DemoAction::CaptureScreenshotAndVideo(filename)
    }

    /// Mark a screenshot as taken.
    pub fn mark_screenshot(&mut self, name: String) {
        self.screenshots.push(name);
    }

    /// Finalize and print summary.
    pub fn finalize(&mut self) {
        if let Ok(mut enc) = self.encoder.lock() {
            enc.finalize();
        }
        tracing::info!("═══════════════════════════════════════════");
        tracing::info!("✅ Feature showcase complete!");
        tracing::info!("   📸 Screenshots:");
        for s in &self.screenshots {
            tracing::info!("      docs/demo/{}", s);
        }
        tracing::info!("   🎬 docs/demo/demo.mp4");
        tracing::info!("═══════════════════════════════════════════");
        self.active = false;
        self.done = true;
    }
}

/// What the demo system should do this frame.
pub enum DemoAction {
    /// Do nothing
    None,
    /// Capture a video frame only
    CaptureVideo,
    /// Set up UI state before capturing (switch panel, open dialogs, etc.)
    SetupUi {
        panel: Option<ActivePanel>,
        setup: SetupAction,
    },
    /// Capture both a PNG screenshot and a video frame
    CaptureScreenshotAndVideo(String),
    /// Finalize and exit
    Finish,
}
