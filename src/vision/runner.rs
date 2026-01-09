//! Vision autosplitter runner

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;

use super::{ScreenCapture, ImageDetector, VisionConfig, VisionTrigger, DetectionType};
use super::capture::CapturedFrame;
use crate::core::SplitEvent;

/// Runner for vision-based autosplitting
pub struct VisionRunner {
    /// Configuration
    config: Arc<RwLock<VisionConfig>>,
    /// Screen capture instance
    capture: ScreenCapture,
    /// Whether the runner is active
    running: Arc<AtomicBool>,
    /// Event callback
    on_split: Option<Box<dyn Fn(SplitEvent) + Send + Sync>>,
}

impl VisionRunner {
    /// Create a new vision runner
    pub fn new(config: VisionConfig) -> Self {
        let capture = if let Some(window) = &config.window_name {
            ScreenCapture::new().with_window(window.clone())
        } else {
            ScreenCapture::new()
        };

        Self {
            config: Arc::new(RwLock::new(config)),
            capture,
            running: Arc::new(AtomicBool::new(false)),
            on_split: None,
        }
    }

    /// Set the split event callback
    pub fn on_split<F>(mut self, callback: F) -> Self
    where
        F: Fn(SplitEvent) + Send + Sync + 'static,
    {
        self.on_split = Some(Box::new(callback));
        self
    }

    /// Start the vision runner
    pub fn start(&self) {
        self.running.store(true, Ordering::SeqCst);
        // Actual implementation would spawn a thread for capture loop
    }

    /// Stop the vision runner
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Process a single frame
    pub fn process_frame(&self, frame: &CapturedFrame) -> Vec<String> {
        let mut triggered = Vec::new();
        let mut config = self.config.write();

        for trigger in &mut config.triggers {
            if trigger.activated {
                continue;
            }

            let matched = match &trigger.detection_type {
                DetectionType::Template { threshold, region, .. } => {
                    // Template matching would go here
                    false
                }
                DetectionType::PixelColor { x, y, color, tolerance } => {
                    ImageDetector::check_pixel(frame, *x, *y, *color, *tolerance)
                }
                DetectionType::MultiPixel { pixels } => {
                    pixels.iter().all(|p| {
                        ImageDetector::check_pixel(frame, p.x, p.y, p.color, p.tolerance)
                    })
                }
            };

            if matched {
                trigger.activated = true;
                triggered.push(trigger.id.clone());

                if let Some(ref callback) = self.on_split {
                    callback(SplitEvent::CustomTrigger {
                        trigger_id: trigger.id.clone(),
                        trigger_name: trigger.name.clone(),
                    });
                }
            }
        }

        triggered
    }

    /// Reset all triggers
    pub fn reset(&self) {
        self.config.write().reset();
    }

    /// Update configuration
    pub fn update_config(&self, config: VisionConfig) {
        *self.config.write() = config;
    }
}
