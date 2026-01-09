//! Vision-based triggers

use serde::{Deserialize, Serialize};
use super::detector::DetectionResult;

/// A trigger based on visual detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionTrigger {
    /// Unique identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Type of detection to perform
    pub detection_type: DetectionType,
    /// Whether this trigger has been activated
    #[serde(default)]
    pub activated: bool,
}

/// Type of visual detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionType {
    /// Template matching with an image
    Template {
        /// Path to template image
        template_path: String,
        /// Confidence threshold
        threshold: f32,
        /// Optional region to search (x, y, w, h)
        region: Option<(u32, u32, u32, u32)>,
    },
    /// Pixel color check
    PixelColor {
        /// X coordinate
        x: u32,
        /// Y coordinate
        y: u32,
        /// Expected RGBA color
        color: [u8; 4],
        /// Color tolerance
        tolerance: u8,
    },
    /// Multiple pixel checks (all must match)
    MultiPixel {
        /// List of pixel checks
        pixels: Vec<PixelCheck>,
    },
}

/// A single pixel color check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PixelCheck {
    pub x: u32,
    pub y: u32,
    pub color: [u8; 4],
    pub tolerance: u8,
}

impl VisionTrigger {
    /// Create a new template-based trigger
    pub fn template(
        id: impl Into<String>,
        name: impl Into<String>,
        template_path: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            detection_type: DetectionType::Template {
                template_path: template_path.into(),
                threshold: 0.9,
                region: None,
            },
            activated: false,
        }
    }

    /// Create a new pixel color trigger
    pub fn pixel(
        id: impl Into<String>,
        name: impl Into<String>,
        x: u32,
        y: u32,
        color: [u8; 4],
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            detection_type: DetectionType::PixelColor {
                x,
                y,
                color,
                tolerance: 10,
            },
            activated: false,
        }
    }

    /// Reset the trigger
    pub fn reset(&mut self) {
        self.activated = false;
    }
}
