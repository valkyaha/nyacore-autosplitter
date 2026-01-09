//! Vision-based autosplitter (feature-gated)
//!
//! This module provides screen capture and image recognition
//! for games that don't support memory reading.
//!
//! Enable with the "vision" feature flag.

mod capture;
mod detector;
mod trigger;
mod config;
mod runner;

pub use capture::ScreenCapture;
pub use detector::{ImageDetector, DetectionResult};
pub use trigger::VisionTrigger;
pub use config::VisionConfig;
pub use runner::VisionRunner;
