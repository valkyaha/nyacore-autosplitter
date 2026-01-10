//! Configuration structures for vision-based autosplitter
//!
//! Defines the TOML schema for console game plugins

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration for a vision-based game plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionConfig {
    pub game: GameInfo,
    pub capture: CaptureConfig,
    #[serde(default)]
    pub triggers: Vec<TriggerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureConfig {
    /// Capture source type: "window", "obs", "screen"
    #[serde(default = "default_source_type")]
    pub source_type: String,
    /// Window title pattern (for window capture)
    pub window_title: Option<String>,
    /// OBS source name (for OBS capture)
    pub obs_source: Option<String>,
    /// Frames per second to analyze (lower = less CPU)
    #[serde(default = "default_fps")]
    pub analysis_fps: u32,
    /// Scale factor for analysis (0.5 = half resolution)
    #[serde(default = "default_scale")]
    pub analysis_scale: f32,
}

fn default_source_type() -> String {
    "window".to_string()
}

fn default_fps() -> u32 {
    4
}

fn default_scale() -> f32 {
    0.5
}

/// Configuration for a single trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    /// Unique identifier for this trigger
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Detection method: "template", "color", "ocr"
    #[serde(rename = "type")]
    pub detector_type: String,
    /// Region of interest (optional, full frame if not specified)
    pub region: Option<RegionConfig>,
    /// Template matching specific config
    pub template: Option<TemplateConfig>,
    /// Color detection specific config
    pub color: Option<ColorConfig>,
    /// OCR specific config
    pub ocr: Option<OcrConfig>,
    /// Action to perform when triggered
    #[serde(default = "default_action")]
    pub action: String,
    /// Cooldown in milliseconds before trigger can fire again
    #[serde(default = "default_cooldown")]
    pub cooldown_ms: u64,
    /// Whether this trigger is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Associated boss/split ID (for linking to splits)
    pub boss_id: Option<String>,
}

fn default_action() -> String {
    "split".to_string()
}

fn default_cooldown() -> u64 {
    5000
}

fn default_enabled() -> bool {
    true
}

/// Screen region definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionConfig {
    /// X position (can be absolute pixels or percentage with %)
    pub x: RegionValue,
    /// Y position
    pub y: RegionValue,
    /// Width
    pub width: RegionValue,
    /// Height
    pub height: RegionValue,
}

/// Region value that can be absolute or percentage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RegionValue {
    Absolute(u32),
    Percentage(String), // "50%" format
}

impl RegionValue {
    /// Resolve to absolute pixels given a dimension
    pub fn resolve(&self, total: u32) -> u32 {
        match self {
            RegionValue::Absolute(v) => *v,
            RegionValue::Percentage(s) => {
                let pct: f32 = s.trim_end_matches('%').parse().unwrap_or(0.0);
                ((pct / 100.0) * total as f32) as u32
            }
        }
    }
}

/// Template matching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    /// Path to template image (relative to plugin directory)
    pub image: PathBuf,
    /// Match threshold (0.0 - 1.0, higher = stricter)
    #[serde(default = "default_threshold")]
    pub threshold: f32,
    /// Use grayscale matching (faster)
    #[serde(default = "default_grayscale")]
    pub grayscale: bool,
}

fn default_threshold() -> f32 {
    0.85
}

fn default_grayscale() -> bool {
    true
}

/// Color detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    /// Target RGB color
    pub target: [u8; 3],
    /// Color tolerance (0-255)
    #[serde(default = "default_tolerance")]
    pub tolerance: u8,
    /// Minimum percentage of region that must match
    #[serde(default = "default_min_match")]
    pub min_match_percent: f32,
    /// Detect when color appears (true) or disappears (false)
    #[serde(default = "default_detect_presence")]
    pub detect_presence: bool,
}

fn default_tolerance() -> u8 {
    15
}

fn default_min_match() -> f32 {
    80.0
}

fn default_detect_presence() -> bool {
    true
}

/// OCR configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    /// Text to search for (case-insensitive by default)
    pub text: String,
    /// Use exact match vs contains
    #[serde(default)]
    pub exact_match: bool,
    /// Case sensitive matching
    #[serde(default)]
    pub case_sensitive: bool,
    /// Language hint for OCR engine
    #[serde(default = "default_language")]
    pub language: String,
    /// Preprocessing: "none", "threshold", "invert"
    #[serde(default = "default_preprocess")]
    pub preprocess: String,
}

fn default_language() -> String {
    "eng".to_string()
}

fn default_preprocess() -> String {
    "threshold".to_string()
}

impl VisionConfig {
    /// Load config from TOML file
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: VisionConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save config to TOML file
    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_value_resolve() {
        let abs = RegionValue::Absolute(100);
        assert_eq!(abs.resolve(1920), 100);

        let pct = RegionValue::Percentage("50%".to_string());
        assert_eq!(pct.resolve(1920), 960);
    }

    #[test]
    fn test_parse_config() {
        let toml = r#"
[game]
id = "bloodborne"
name = "Bloodborne"

[capture]
source_type = "window"
window_title = "OBS"
analysis_fps = 4

[[triggers]]
id = "nightmare_slain"
name = "Nightmare Slain"
type = "template"
action = "split"
cooldown_ms = 5000

[triggers.region]
x = "30%"
y = "40%"
width = "40%"
height = "20%"

[triggers.template]
image = "templates/nightmare_slain.png"
threshold = 0.85
"#;
        let config: VisionConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.game.id, "bloodborne");
        assert_eq!(config.triggers.len(), 1);
    }
}
