//! Vision autosplitter runner
//!
//! Main loop that captures frames and processes triggers

use super::capture::{create_capture_source, CaptureSource};
use super::config::VisionConfig;
use super::trigger::{TriggerEvent, TriggerManager};
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Test mode configuration
#[derive(Debug, Clone)]
pub struct TestModeConfig {
    /// Path to video file or frame sequence directory
    pub source_path: PathBuf,
    /// Whether source is a video file or directory of frames
    pub is_video: bool,
    /// Loop the video/sequence
    pub loop_playback: bool,
    /// Playback speed multiplier (1.0 = normal, 2.0 = 2x speed)
    pub playback_speed: f32,
}

/// State of the vision autosplitter
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisionState {
    /// Not running
    Stopped,
    /// Waiting for capture source
    WaitingForSource,
    /// Running and processing frames
    Running,
    /// Error state
    Error(String),
}

/// Callback for trigger events
pub type TriggerCallback = Arc<dyn Fn(TriggerEvent) + Send + Sync>;

/// Vision-based autosplitter
pub struct VisionAutosplitter {
    /// Current state
    state: Arc<Mutex<VisionState>>,
    /// Running flag
    running: Arc<AtomicBool>,
    /// Worker thread handle
    worker: Option<JoinHandle<()>>,
    /// Trigger callback
    callback: Option<TriggerCallback>,
    /// Current config
    config: Arc<Mutex<Option<VisionConfig>>>,
    /// Plugin base path (for loading templates)
    plugin_path: Arc<Mutex<Option<PathBuf>>>,
    /// Test mode configuration (if in test mode)
    test_mode: Arc<Mutex<Option<TestModeConfig>>>,
}

impl VisionAutosplitter {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(VisionState::Stopped)),
            running: Arc::new(AtomicBool::new(false)),
            worker: None,
            callback: None,
            config: Arc::new(Mutex::new(None)),
            plugin_path: Arc::new(Mutex::new(None)),
            test_mode: Arc::new(Mutex::new(None)),
        }
    }

    /// Set callback for trigger events
    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: Fn(TriggerEvent) + Send + Sync + 'static,
    {
        self.callback = Some(Arc::new(callback));
    }

    /// Start the vision autosplitter with config
    pub fn start(&mut self, config: VisionConfig, plugin_path: PathBuf) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Err(anyhow!("Vision autosplitter already running"));
        }

        // Clear test mode
        *self.test_mode.lock().unwrap() = None;

        // Store config
        *self.config.lock().unwrap() = Some(config.clone());
        *self.plugin_path.lock().unwrap() = Some(plugin_path.clone());

        // Set running
        self.running.store(true, Ordering::SeqCst);
        *self.state.lock().unwrap() = VisionState::WaitingForSource;

        // Clone for thread
        let running = self.running.clone();
        let state = self.state.clone();
        let callback = self.callback.clone();

        // Spawn worker thread
        let handle = thread::Builder::new()
            .name("vision-autosplitter".to_string())
            .spawn(move || {
                run_vision_loop(running, state, config, plugin_path, callback, None);
            })?;

        self.worker = Some(handle);

        log::info!("Vision autosplitter started");
        Ok(())
    }

    /// Start the vision autosplitter in test mode with video file or frame sequence
    pub fn start_test_mode(
        &mut self,
        config: VisionConfig,
        plugin_path: PathBuf,
        test_config: TestModeConfig,
    ) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Err(anyhow!("Vision autosplitter already running"));
        }

        // Store test mode config
        *self.test_mode.lock().unwrap() = Some(test_config.clone());

        // Store config
        *self.config.lock().unwrap() = Some(config.clone());
        *self.plugin_path.lock().unwrap() = Some(plugin_path.clone());

        // Set running
        self.running.store(true, Ordering::SeqCst);
        *self.state.lock().unwrap() = VisionState::Running; // Start directly in Running state for test mode

        // Clone for thread
        let running = self.running.clone();
        let state = self.state.clone();
        let callback = self.callback.clone();

        // Spawn worker thread with test capture source
        let handle = thread::Builder::new()
            .name("vision-autosplitter-test".to_string())
            .spawn(move || {
                run_vision_loop(running, state, config, plugin_path, callback, Some(test_config));
            })?;

        self.worker = Some(handle);

        log::info!("Vision autosplitter started in test mode");
        Ok(())
    }

    /// Check if running in test mode
    pub fn is_test_mode(&self) -> bool {
        self.test_mode.lock().unwrap().is_some()
    }

    /// Stop the vision autosplitter
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);

        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }

        *self.state.lock().unwrap() = VisionState::Stopped;
        log::info!("Vision autosplitter stopped");
    }

    /// Reset triggers (for new run)
    pub fn reset(&mut self) {
        // Will be handled by the loop when it detects reset flag
        // For now just log
        log::info!("Vision autosplitter reset requested");
    }

    /// Get current state
    pub fn state(&self) -> VisionState {
        self.state.lock().unwrap().clone()
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get current config
    pub fn config(&self) -> Option<VisionConfig> {
        self.config.lock().unwrap().clone()
    }
}

impl Default for VisionAutosplitter {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for VisionAutosplitter {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Main vision processing loop
fn run_vision_loop(
    running: Arc<AtomicBool>,
    state: Arc<Mutex<VisionState>>,
    config: VisionConfig,
    plugin_path: PathBuf,
    callback: Option<TriggerCallback>,
    test_mode: Option<TestModeConfig>,
) {
    let frame_interval = Duration::from_millis(1000 / config.capture.analysis_fps.max(1) as u64);

    // Create capture source based on mode
    let mut capture: Option<Box<dyn CaptureSource>> = None;
    let mut last_source_check = Instant::now();
    let source_check_interval = Duration::from_secs(2);

    // In test mode, create capture source immediately
    if let Some(ref test_config) = test_mode {
        match create_test_capture_source(test_config) {
            Ok(src) => {
                log::info!("Test capture source created: {:?}", test_config.source_path);
                capture = Some(src);
            }
            Err(e) => {
                log::error!("Failed to create test capture source: {}", e);
                *state.lock().unwrap() = VisionState::Error(format!("Test capture failed: {}", e));
                return;
            }
        }
    }

    // Load triggers
    let trigger_manager = match TriggerManager::load_from_configs(&config.triggers, &plugin_path) {
        Ok(tm) => tm,
        Err(e) => {
            log::error!("Failed to load triggers: {}", e);
            *state.lock().unwrap() = VisionState::Error(format!("Failed to load triggers: {}", e));
            return;
        }
    };
    let mut trigger_manager = trigger_manager;

    log::info!(
        "Vision autosplitter loaded {} triggers",
        trigger_manager.len()
    );

    let mut frame_count: u64 = 0;
    let mut last_log = Instant::now();
    let is_test_mode = test_mode.is_some();

    while running.load(Ordering::SeqCst) {
        let frame_start = Instant::now();

        // Try to get/reconnect capture source (only in normal mode)
        if !is_test_mode && (capture.is_none() || !capture.as_ref().unwrap().is_available()) {
            if last_source_check.elapsed() >= source_check_interval {
                last_source_check = Instant::now();

                match create_capture_source(
                    &config.capture.source_type,
                    config.capture.window_title.as_deref(),
                    config.capture.obs_source.as_deref(),
                ) {
                    Ok(src) => {
                        log::info!("Capture source connected");
                        capture = Some(src);
                        *state.lock().unwrap() = VisionState::Running;
                    }
                    Err(e) => {
                        log::debug!("Waiting for capture source: {}", e);
                        *state.lock().unwrap() = VisionState::WaitingForSource;
                    }
                }
            }

            // Wait before next attempt
            thread::sleep(Duration::from_millis(100));
            continue;
        }

        // Check if capture source is available
        if capture.is_none() {
            thread::sleep(Duration::from_millis(100));
            continue;
        }

        // Capture frame
        let frame = match capture.as_mut().unwrap().capture() {
            Ok(f) => f,
            Err(e) => {
                if is_test_mode {
                    log::info!("Test video ended: {}", e);
                    *state.lock().unwrap() = VisionState::Stopped;
                    break;
                }
                log::warn!("Capture failed: {}", e);
                capture = None;
                continue;
            }
        };

        frame_count += 1;

        // Log frame stats every 5 seconds
        if last_log.elapsed() >= Duration::from_secs(5) {
            let mode_str = if is_test_mode { " (TEST MODE)" } else { "" };
            log::info!(
                "Vision{}: {} frames captured, {}x{} resolution",
                mode_str,
                frame_count,
                frame.width,
                frame.height
            );
            last_log = Instant::now();
        }

        // Scale if needed
        let frame = if config.capture.analysis_scale < 1.0 {
            frame.scale(config.capture.analysis_scale)
        } else {
            frame
        };

        // Process triggers
        let events = trigger_manager.process(&frame);

        // Fire callbacks
        if let Some(ref cb) = callback {
            for event in events {
                cb(event);
            }
        }

        // Maintain frame rate (unless in test mode with playback speed control)
        let elapsed = frame_start.elapsed();
        if elapsed < frame_interval {
            thread::sleep(frame_interval - elapsed);
        }
    }
}

/// Create capture source for test mode
fn create_test_capture_source(test_config: &TestModeConfig) -> anyhow::Result<Box<dyn CaptureSource>> {
    use super::capture::FrameSequenceCapture;

    if test_config.is_video {
        #[cfg(feature = "video-test")]
        {
            use super::capture::video_capture::VideoCapture;
            let capture = VideoCapture::open(
                &test_config.source_path,
                test_config.loop_playback,
                test_config.playback_speed,
            )?;
            Ok(Box::new(capture))
        }
        #[cfg(not(feature = "video-test"))]
        {
            Err(anyhow!("Video test mode requires 'video-test' feature. Build with: cargo build --features video-test"))
        }
    } else {
        // Frame sequence mode (directory of images)
        let capture = FrameSequenceCapture::from_directory(
            &test_config.source_path,
            test_config.loop_playback,
        )?;
        Ok(Box::new(capture))
    }
}

/// Builder for VisionAutosplitter
pub struct VisionAutosplitterBuilder {
    callback: Option<TriggerCallback>,
}

impl VisionAutosplitterBuilder {
    pub fn new() -> Self {
        Self { callback: None }
    }

    pub fn on_trigger<F>(mut self, callback: F) -> Self
    where
        F: Fn(TriggerEvent) + Send + Sync + 'static,
    {
        self.callback = Some(Arc::new(callback));
        self
    }

    pub fn build(self) -> VisionAutosplitter {
        let mut vs = VisionAutosplitter::new();
        vs.callback = self.callback;
        vs
    }
}

impl Default for VisionAutosplitterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vision_autosplitter_new() {
        let vs = VisionAutosplitter::new();
        assert_eq!(vs.state(), VisionState::Stopped);
        assert!(!vs.is_running());
    }
}
