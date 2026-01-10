//! Vision-based autosplitter for console games
//!
//! This module provides video capture and image analysis capabilities
//! for detecting game events (boss kills, loading screens, etc.) without
//! requiring memory access - perfect for console games via capture cards.
//!
//! # Example
//!
//! ```ignore
//! use autosplitter::vision::{VisionAutosplitter, VisionConfig};
//!
//! let config = VisionConfig::load("plugins/bloodborne/vision_plugin.toml")?;
//! let mut splitter = VisionAutosplitter::new();
//! splitter.set_callback(|event| {
//!     println!("Trigger fired: {}", event.trigger_name);
//! });
//! splitter.start(config, plugin_path)?;
//! ```

pub mod capture;
pub mod config;
pub mod detector;
pub mod trigger;

mod runner;

// Re-export main types for convenient access
#[allow(unused_imports)]
pub use capture::{CaptureSource, FileCapture, FrameData, FrameSequenceCapture};
pub use config::VisionConfig;
#[allow(unused_imports)]
pub use detector::{DetectionResult, Detector, DetectorType};
pub use runner::{TestModeConfig, VisionAutosplitter};
#[allow(unused_imports)]
pub use trigger::{TriggerAction, TriggerEvent, VisionTrigger};

#[cfg(feature = "video-test")]
pub use capture::video_capture::VideoCapture;
