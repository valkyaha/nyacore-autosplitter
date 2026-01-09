//! Image detection algorithms

use super::capture::CapturedFrame;

/// Result of a detection operation
#[derive(Debug, Clone)]
pub struct DetectionResult {
    /// Whether a match was found
    pub found: bool,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f32,
    /// Location of the match (x, y) if found
    pub location: Option<(u32, u32)>,
}

impl DetectionResult {
    /// Create a not-found result
    pub fn not_found() -> Self {
        Self {
            found: false,
            confidence: 0.0,
            location: None,
        }
    }

    /// Create a found result
    pub fn found(confidence: f32, x: u32, y: u32) -> Self {
        Self {
            found: true,
            confidence,
            location: Some((x, y)),
        }
    }
}

/// Image detection using template matching
pub struct ImageDetector {
    /// Template image data
    template: Option<CapturedFrame>,
    /// Minimum confidence threshold
    threshold: f32,
}

impl ImageDetector {
    /// Create a new detector
    pub fn new() -> Self {
        Self {
            template: None,
            threshold: 0.9,
        }
    }

    /// Set the template image
    pub fn with_template(mut self, template: CapturedFrame) -> Self {
        self.template = Some(template);
        self
    }

    /// Set the confidence threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Detect the template in a frame
    pub fn detect(&self, frame: &CapturedFrame) -> DetectionResult {
        let Some(template) = &self.template else {
            return DetectionResult::not_found();
        };

        // Placeholder - actual implementation would use image processing
        // This would typically use normalized cross-correlation or similar
        DetectionResult::not_found()
    }

    /// Check if a specific pixel matches expected color
    pub fn check_pixel(
        frame: &CapturedFrame,
        x: u32,
        y: u32,
        expected: [u8; 4],
        tolerance: u8,
    ) -> bool {
        if let Some(pixel) = frame.get_pixel(x, y) {
            let dr = (pixel[0] as i16 - expected[0] as i16).abs();
            let dg = (pixel[1] as i16 - expected[1] as i16).abs();
            let db = (pixel[2] as i16 - expected[2] as i16).abs();
            dr <= tolerance as i16 && dg <= tolerance as i16 && db <= tolerance as i16
        } else {
            false
        }
    }
}

impl Default for ImageDetector {
    fn default() -> Self {
        Self::new()
    }
}
