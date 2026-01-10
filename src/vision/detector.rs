//! Detection algorithms for vision-based autosplitter
//!
//! Supports template matching, color detection, and OCR

use super::capture::FrameData;
use super::config::{ColorConfig, OcrConfig, RegionConfig, RegionValue, TemplateConfig};
use anyhow::{anyhow, Result};
use image::{DynamicImage, GrayImage, RgbaImage};
use std::path::Path;

/// Result of a detection attempt
#[derive(Debug, Clone)]
pub struct DetectionResult {
    /// Whether the detection matched
    pub matched: bool,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Detection type that matched
    pub detector_type: DetectorType,
    /// Optional location of match (x, y)
    pub location: Option<(u32, u32)>,
}

impl DetectionResult {
    pub fn no_match(detector_type: DetectorType) -> Self {
        Self {
            matched: false,
            confidence: 0.0,
            detector_type,
            location: None,
        }
    }

    pub fn matched(detector_type: DetectorType, confidence: f32) -> Self {
        Self {
            matched: true,
            confidence,
            detector_type,
            location: None,
        }
    }

    pub fn matched_at(detector_type: DetectorType, confidence: f32, x: u32, y: u32) -> Self {
        Self {
            matched: true,
            confidence,
            detector_type,
            location: Some((x, y)),
        }
    }
}

/// Type of detector
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectorType {
    Template,
    Color,
    Ocr,
}

impl std::fmt::Display for DetectorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectorType::Template => write!(f, "template"),
            DetectorType::Color => write!(f, "color"),
            DetectorType::Ocr => write!(f, "ocr"),
        }
    }
}

impl std::str::FromStr for DetectorType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "template" => Ok(DetectorType::Template),
            "color" => Ok(DetectorType::Color),
            "ocr" => Ok(DetectorType::Ocr),
            _ => Err(anyhow!("Unknown detector type: {}", s)),
        }
    }
}

/// Trait for all detectors
pub trait Detector: Send + Sync {
    /// Run detection on a frame
    fn detect(&self, frame: &FrameData) -> Result<DetectionResult>;

    /// Get the detector type
    fn detector_type(&self) -> DetectorType;
}

/// Template matching detector
pub struct TemplateDetector {
    template: GrayImage,
    template_color: Option<RgbaImage>,
    threshold: f32,
    use_grayscale: bool,
    region: Option<ResolvedRegion>,
}

#[derive(Debug, Clone)]
struct ResolvedRegion {
    x_pct: Option<f32>,
    y_pct: Option<f32>,
    w_pct: Option<f32>,
    h_pct: Option<f32>,
    x_abs: Option<u32>,
    y_abs: Option<u32>,
    w_abs: Option<u32>,
    h_abs: Option<u32>,
}

impl ResolvedRegion {
    fn from_config(config: &RegionConfig) -> Self {
        let parse_value = |v: &RegionValue| -> (Option<f32>, Option<u32>) {
            match v {
                RegionValue::Percentage(s) => {
                    let pct: f32 = s.trim_end_matches('%').parse().unwrap_or(0.0);
                    (Some(pct / 100.0), None)
                }
                RegionValue::Absolute(abs) => (None, Some(*abs)),
            }
        };

        let (x_pct, x_abs) = parse_value(&config.x);
        let (y_pct, y_abs) = parse_value(&config.y);
        let (w_pct, w_abs) = parse_value(&config.width);
        let (h_pct, h_abs) = parse_value(&config.height);

        Self {
            x_pct,
            y_pct,
            w_pct,
            h_pct,
            x_abs,
            y_abs,
            w_abs,
            h_abs,
        }
    }

    fn resolve(&self, frame_width: u32, frame_height: u32) -> (u32, u32, u32, u32) {
        let x = self
            .x_abs
            .unwrap_or_else(|| (self.x_pct.unwrap_or(0.0) * frame_width as f32) as u32);
        let y = self
            .y_abs
            .unwrap_or_else(|| (self.y_pct.unwrap_or(0.0) * frame_height as f32) as u32);
        let w = self
            .w_abs
            .unwrap_or_else(|| (self.w_pct.unwrap_or(1.0) * frame_width as f32) as u32);
        let h = self
            .h_abs
            .unwrap_or_else(|| (self.h_pct.unwrap_or(1.0) * frame_height as f32) as u32);
        (x, y, w, h)
    }
}

impl TemplateDetector {
    /// Create from config
    pub fn new(config: &TemplateConfig, base_path: &Path, region: Option<&RegionConfig>) -> Result<Self> {
        let template_path = base_path.join(&config.image);
        let template_img = image::open(&template_path)
            .map_err(|e| anyhow!("Failed to load template {}: {}", template_path.display(), e))?;

        let template = template_img.to_luma8();
        let template_color = if config.grayscale {
            None
        } else {
            Some(template_img.to_rgba8())
        };

        Ok(Self {
            template,
            template_color,
            threshold: config.threshold,
            use_grayscale: config.grayscale,
            region: region.map(ResolvedRegion::from_config),
        })
    }

    /// Create from image directly (for testing)
    pub fn from_image(template: DynamicImage, threshold: f32) -> Self {
        Self {
            template: template.to_luma8(),
            template_color: None,
            threshold,
            use_grayscale: true,
            region: None,
        }
    }
}

impl Detector for TemplateDetector {
    fn detect(&self, frame: &FrameData) -> Result<DetectionResult> {
        let frame_img = frame
            .to_image()
            .ok_or_else(|| anyhow!("Failed to convert frame to image"))?;

        // Crop to region if specified
        let search_img = if let Some(ref region) = self.region {
            let (x, y, w, h) = region.resolve(frame.width, frame.height);
            frame_img.crop_imm(x, y, w, h)
        } else {
            frame_img
        };

        let search_gray = search_img.to_luma8();

        // Ensure template fits in search region
        if self.template.width() > search_gray.width()
            || self.template.height() > search_gray.height()
        {
            log::warn!(
                "Template too large: template={}x{}, search={}x{}",
                self.template.width(),
                self.template.height(),
                search_gray.width(),
                search_gray.height()
            );
            return Ok(DetectionResult::no_match(DetectorType::Template));
        }

        // Perform template matching using normalized cross-correlation
        let result = template_match_ncc(&search_gray, &self.template);

        // Log confidence for debugging every time (temporarily for debugging)
        log::info!(
            "Template match: confidence={:.3}, threshold={:.2}, template={}x{}, search={}x{}",
            result.0,
            self.threshold,
            self.template.width(),
            self.template.height(),
            search_gray.width(),
            search_gray.height()
        );

        if result.0 >= self.threshold {
            Ok(DetectionResult::matched_at(
                DetectorType::Template,
                result.0,
                result.1,
                result.2,
            ))
        } else {
            Ok(DetectionResult {
                matched: false,
                confidence: result.0,
                detector_type: DetectorType::Template,
                location: None,
            })
        }
    }

    fn detector_type(&self) -> DetectorType {
        DetectorType::Template
    }
}

/// Normalized Cross-Correlation template matching with stride optimization
/// Returns (max_correlation, x, y)
fn template_match_ncc(image: &GrayImage, template: &GrayImage) -> (f32, u32, u32) {
    let (img_w, img_h) = image.dimensions();
    let (tpl_w, tpl_h) = template.dimensions();

    if tpl_w > img_w || tpl_h > img_h {
        return (0.0, 0, 0);
    }

    let start = std::time::Instant::now();
    let positions = ((img_w - tpl_w + 1) as u64) * ((img_h - tpl_h + 1) as u64);
    let ops_per_pos = (tpl_w as u64) * (tpl_h as u64);
    log::debug!(
        "Template match starting: {}x{} template on {}x{} image ({} positions, {} ops each)",
        tpl_w, tpl_h, img_w, img_h, positions, ops_per_pos
    );

    // Pre-compute template mean and std
    let tpl_mean = template_mean(template);
    let tpl_std = template_std(template, tpl_mean);

    if tpl_std < 1e-6 {
        return (0.0, 0, 0);
    }

    let mut max_corr = f32::MIN;
    let mut max_x = 0u32;
    let mut max_y = 0u32;

    // Use stride to reduce search space - check every Nth pixel first, then refine
    // Stride of 4 = 16x speedup in coarse search
    let coarse_stride = 4u32;

    // Coarse search
    let mut y = 0;
    while y <= img_h - tpl_h {
        let mut x = 0;
        while x <= img_w - tpl_w {
            let corr = compute_ncc(image, template, x, y, tpl_mean, tpl_std);
            if corr > max_corr {
                max_corr = corr;
                max_x = x;
                max_y = y;
            }
            x += coarse_stride;
        }
        y += coarse_stride;
    }

    // Fine search around best match (if we found something promising)
    if max_corr > 0.3 {
        let search_radius = coarse_stride;
        let start_y = max_y.saturating_sub(search_radius);
        let end_y = (max_y + search_radius).min(img_h - tpl_h);
        let start_x = max_x.saturating_sub(search_radius);
        let end_x = (max_x + search_radius).min(img_w - tpl_w);

        for y in start_y..=end_y {
            for x in start_x..=end_x {
                let corr = compute_ncc(image, template, x, y, tpl_mean, tpl_std);
                if corr > max_corr {
                    max_corr = corr;
                    max_x = x;
                    max_y = y;
                }
            }
        }
    }

    let elapsed = start.elapsed();
    log::debug!(
        "Template match completed in {:?}: max_corr={:.3} at ({}, {})",
        elapsed, max_corr, max_x, max_y
    );

    (max_corr.max(0.0), max_x, max_y)
}

fn template_mean(template: &GrayImage) -> f32 {
    let sum: f32 = template.pixels().map(|p| p.0[0] as f32).sum();
    sum / (template.width() * template.height()) as f32
}

fn template_std(template: &GrayImage, mean: f32) -> f32 {
    let variance: f32 = template
        .pixels()
        .map(|p| {
            let diff = p.0[0] as f32 - mean;
            diff * diff
        })
        .sum();
    (variance / (template.width() * template.height()) as f32).sqrt()
}

fn compute_ncc(
    image: &GrayImage,
    template: &GrayImage,
    offset_x: u32,
    offset_y: u32,
    tpl_mean: f32,
    tpl_std: f32,
) -> f32 {
    let (tpl_w, tpl_h) = template.dimensions();

    // Compute patch mean
    let mut patch_sum: f32 = 0.0;
    for ty in 0..tpl_h {
        for tx in 0..tpl_w {
            patch_sum += image.get_pixel(offset_x + tx, offset_y + ty).0[0] as f32;
        }
    }
    let patch_mean = patch_sum / (tpl_w * tpl_h) as f32;

    // Compute patch std
    let mut patch_var: f32 = 0.0;
    for ty in 0..tpl_h {
        for tx in 0..tpl_w {
            let diff = image.get_pixel(offset_x + tx, offset_y + ty).0[0] as f32 - patch_mean;
            patch_var += diff * diff;
        }
    }
    let patch_std = (patch_var / (tpl_w * tpl_h) as f32).sqrt();

    if patch_std < 1e-6 {
        return 0.0;
    }

    // Compute NCC
    let mut ncc: f32 = 0.0;
    for ty in 0..tpl_h {
        for tx in 0..tpl_w {
            let img_val = image.get_pixel(offset_x + tx, offset_y + ty).0[0] as f32;
            let tpl_val = template.get_pixel(tx, ty).0[0] as f32;
            ncc += (img_val - patch_mean) * (tpl_val - tpl_mean);
        }
    }

    ncc / ((tpl_w * tpl_h) as f32 * patch_std * tpl_std)
}

/// Color detection detector
pub struct ColorDetector {
    target: [u8; 3],
    tolerance: u8,
    min_match_percent: f32,
    detect_presence: bool,
    region: Option<ResolvedRegion>,
}

impl ColorDetector {
    pub fn new(config: &ColorConfig, region: Option<&RegionConfig>) -> Self {
        Self {
            target: config.target,
            tolerance: config.tolerance,
            min_match_percent: config.min_match_percent,
            detect_presence: config.detect_presence,
            region: region.map(ResolvedRegion::from_config),
        }
    }

    fn color_matches(&self, r: u8, g: u8, b: u8) -> bool {
        let dr = (r as i16 - self.target[0] as i16).abs();
        let dg = (g as i16 - self.target[1] as i16).abs();
        let db = (b as i16 - self.target[2] as i16).abs();
        dr <= self.tolerance as i16
            && dg <= self.tolerance as i16
            && db <= self.tolerance as i16
    }
}

impl Detector for ColorDetector {
    fn detect(&self, frame: &FrameData) -> Result<DetectionResult> {
        let (start_x, start_y, width, height) = if let Some(ref region) = self.region {
            region.resolve(frame.width, frame.height)
        } else {
            (0, 0, frame.width, frame.height)
        };

        let mut matching_pixels = 0u32;
        let total_pixels = width * height;

        for y in start_y..(start_y + height).min(frame.height) {
            for x in start_x..(start_x + width).min(frame.width) {
                let idx = ((y * frame.width + x) * 4) as usize;
                if idx + 2 < frame.pixels.len() {
                    let r = frame.pixels[idx];
                    let g = frame.pixels[idx + 1];
                    let b = frame.pixels[idx + 2];
                    if self.color_matches(r, g, b) {
                        matching_pixels += 1;
                    }
                }
            }
        }

        let match_percent = (matching_pixels as f32 / total_pixels as f32) * 100.0;
        let threshold_met = match_percent >= self.min_match_percent;
        let detected = if self.detect_presence {
            threshold_met
        } else {
            !threshold_met
        };

        if detected {
            Ok(DetectionResult::matched(
                DetectorType::Color,
                match_percent / 100.0,
            ))
        } else {
            Ok(DetectionResult {
                matched: false,
                confidence: match_percent / 100.0,
                detector_type: DetectorType::Color,
                location: None,
            })
        }
    }

    fn detector_type(&self) -> DetectorType {
        DetectorType::Color
    }
}

/// OCR detector using Windows Media OCR API with Tesseract fallback
pub struct OcrDetector {
    target_text: String,
    exact_match: bool,
    case_sensitive: bool,
    region: Option<ResolvedRegion>,
    preprocess: String,
    language: String,
}

impl OcrDetector {
    pub fn new(config: &OcrConfig, region: Option<&RegionConfig>) -> Self {
        Self {
            target_text: if config.case_sensitive {
                config.text.clone()
            } else {
                config.text.to_lowercase()
            },
            exact_match: config.exact_match,
            case_sensitive: config.case_sensitive,
            region: region.map(ResolvedRegion::from_config),
            preprocess: config.preprocess.clone(),
            language: config.language.clone(),
        }
    }

    /// Check if detected text matches the target
    fn text_matches(&self, detected: &str) -> bool {
        let detected_normalized = if self.case_sensitive {
            detected.to_string()
        } else {
            detected.to_lowercase()
        };

        if self.exact_match {
            detected_normalized.trim() == self.target_text
        } else {
            detected_normalized.contains(&self.target_text)
        }
    }

    /// Preprocess image for better OCR results
    fn preprocess_image(&self, img: &GrayImage) -> GrayImage {
        match self.preprocess.as_str() {
            "threshold" => {
                // Apply Otsu's thresholding for binary image
                let threshold = otsu_threshold(img);
                let mut result = img.clone();
                for pixel in result.pixels_mut() {
                    pixel.0[0] = if pixel.0[0] > threshold { 255 } else { 0 };
                }
                result
            }
            "invert" => {
                // Invert colors (useful for light text on dark background)
                let mut result = img.clone();
                for pixel in result.pixels_mut() {
                    pixel.0[0] = 255 - pixel.0[0];
                }
                result
            }
            "threshold_invert" => {
                // Threshold then invert
                let threshold = otsu_threshold(img);
                let mut result = img.clone();
                for pixel in result.pixels_mut() {
                    pixel.0[0] = if pixel.0[0] > threshold { 0 } else { 255 };
                }
                result
            }
            _ => img.clone(), // "none" or unknown
        }
    }
}

/// Calculate Otsu's threshold for binarization
fn otsu_threshold(img: &GrayImage) -> u8 {
    let mut histogram = [0u32; 256];
    for pixel in img.pixels() {
        histogram[pixel.0[0] as usize] += 1;
    }

    let total = img.width() * img.height();
    let mut sum: f64 = 0.0;
    for (i, &count) in histogram.iter().enumerate() {
        sum += i as f64 * count as f64;
    }

    let mut sum_b: f64 = 0.0;
    let mut w_b: u32 = 0;
    let mut max_variance: f64 = 0.0;
    let mut threshold: u8 = 0;

    for (i, &count) in histogram.iter().enumerate() {
        w_b += count;
        if w_b == 0 {
            continue;
        }

        let w_f = total - w_b;
        if w_f == 0 {
            break;
        }

        sum_b += i as f64 * count as f64;

        let m_b = sum_b / w_b as f64;
        let m_f = (sum - sum_b) / w_f as f64;

        let variance = w_b as f64 * w_f as f64 * (m_b - m_f) * (m_b - m_f);

        if variance > max_variance {
            max_variance = variance;
            threshold = i as u8;
        }
    }

    threshold
}

#[cfg(windows)]
mod windows_ocr {
    use super::*;
    use anyhow::{anyhow, Result};

    // COM interface GUID for IMemoryBufferByteAccess
    const IID_IMEMORY_BUFFER_BYTE_ACCESS: windows::core::GUID =
        windows::core::GUID::from_u128(0x5b0d3235_4dba_4d44_865e_8f1d0e4fd04d);

    #[repr(C)]
    struct IMemoryBufferByteAccessVtbl {
        base: windows::core::IUnknown_Vtbl,
        get_buffer: unsafe extern "system" fn(
            this: *mut std::ffi::c_void,
            value: *mut *mut u8,
            capacity: *mut u32,
        ) -> windows::core::HRESULT,
    }

    /// Perform OCR using Windows Media OCR API
    pub fn recognize_text(img: &GrayImage) -> Result<String> {
        use windows::Graphics::Imaging::{BitmapBufferAccessMode, BitmapPixelFormat, SoftwareBitmap};
        use windows::Media::Ocr::OcrEngine;

        let width = img.width();
        let height = img.height();

        // Create bitmap
        let bitmap = SoftwareBitmap::Create(
            BitmapPixelFormat::Bgra8,
            width as i32,
            height as i32,
        )?;

        // Copy pixel data using LockBuffer
        {
            let buffer = bitmap.LockBuffer(BitmapBufferAccessMode::Write)?;
            let plane = buffer.GetPlaneDescription(0)?;
            let reference = buffer.CreateReference()?;

            unsafe {
                use windows::core::Interface;

                // Query for IMemoryBufferByteAccess interface
                let mut byte_access: *mut std::ffi::c_void = std::ptr::null_mut();
                reference.query(&IID_IMEMORY_BUFFER_BYTE_ACCESS, &mut byte_access).ok()?;

                if byte_access.is_null() {
                    return Err(anyhow!("Failed to get IMemoryBufferByteAccess"));
                }

                // Get the vtable and call GetBuffer
                let vtbl = *(byte_access as *const *const IMemoryBufferByteAccessVtbl);
                let mut data_ptr: *mut u8 = std::ptr::null_mut();
                let mut capacity: u32 = 0;

                let hr = ((*vtbl).get_buffer)(byte_access, &mut data_ptr, &mut capacity);
                if hr.is_err() {
                    return Err(anyhow!("GetBuffer failed: {:?}", hr));
                }

                let stride = plane.Stride as u32;
                for y in 0..height {
                    for x in 0..width {
                        let gray = img.get_pixel(x, y).0[0];
                        let offset = (y * stride + x * 4) as isize;
                        *data_ptr.offset(offset) = gray;     // B
                        *data_ptr.offset(offset + 1) = gray; // G
                        *data_ptr.offset(offset + 2) = gray; // R
                        *data_ptr.offset(offset + 3) = 255;  // A
                    }
                }
                // COM reference is released when `reference` goes out of scope
            }
        }

        // Create OCR engine for the user's preferred language
        let engine = OcrEngine::TryCreateFromUserProfileLanguages()?;

        // Perform OCR
        let result = engine.RecognizeAsync(&bitmap)?.get()?;

        Ok(result.Text()?.to_string())
    }
}

#[cfg(not(windows))]
mod windows_ocr {
    use super::*;
    use anyhow::{anyhow, Result};

    pub fn recognize_text(_img: &GrayImage) -> Result<String> {
        Err(anyhow!("Windows OCR not available on this platform"))
    }
}

/// Tesseract OCR fallback (requires tesseract-ocr feature and Tesseract installed)
#[cfg(feature = "tesseract-ocr")]
mod tesseract_ocr {
    use super::*;
    use anyhow::Result;

    /// Perform OCR using Tesseract
    pub fn recognize_text(img: &GrayImage, language: &str) -> Result<String> {
        use tesseract::Tesseract;

        // Convert GrayImage to raw bytes
        let width = img.width() as i32;
        let height = img.height() as i32;
        let bytes_per_pixel = 1; // Grayscale
        let bytes_per_line = width * bytes_per_pixel;

        // Get raw pixel data
        let raw_data: Vec<u8> = img.as_raw().clone();

        // Create Tesseract instance
        let mut tess = Tesseract::new(None, Some(language))
            .map_err(|e| anyhow::anyhow!("Failed to initialize Tesseract: {}", e))?;

        // Set image from raw bytes
        tess = tess
            .set_image_from_mem(&raw_data)
            .map_err(|e| anyhow::anyhow!("Failed to set image: {}", e))?;

        // Recognize text
        let text = tess
            .get_text()
            .map_err(|e| anyhow::anyhow!("OCR failed: {}", e))?;

        Ok(text)
    }
}

#[cfg(not(feature = "tesseract-ocr"))]
mod tesseract_ocr {
    use super::*;
    use anyhow::{anyhow, Result};

    pub fn recognize_text(_img: &GrayImage, _language: &str) -> Result<String> {
        Err(anyhow!("Tesseract OCR not available. Enable 'tesseract-ocr' feature and install Tesseract."))
    }
}

impl Detector for OcrDetector {
    fn detect(&self, frame: &FrameData) -> Result<DetectionResult> {
        let frame_img = frame
            .to_image()
            .ok_or_else(|| anyhow!("Failed to convert frame to image"))?;

        // Crop to region if specified
        let search_img = if let Some(ref region) = self.region {
            let (x, y, w, h) = region.resolve(frame.width, frame.height);
            frame_img.crop_imm(x, y, w, h)
        } else {
            frame_img
        };

        // Convert to grayscale and preprocess
        let gray = search_img.to_luma8();
        let processed = self.preprocess_image(&gray);

        // Try Windows OCR first (on Windows), then fall back to Tesseract
        let ocr_result = windows_ocr::recognize_text(&processed)
            .or_else(|win_err| {
                log::debug!("Windows OCR unavailable ({}), trying Tesseract...", win_err);
                tesseract_ocr::recognize_text(&processed, &self.language)
            });

        match ocr_result {
            Ok(detected_text) => {
                let matched = self.text_matches(&detected_text);

                log::debug!(
                    "OCR: detected='{}', target='{}', matched={}",
                    detected_text.trim(),
                    self.target_text,
                    matched
                );

                if matched {
                    Ok(DetectionResult::matched(DetectorType::Ocr, 1.0))
                } else {
                    Ok(DetectionResult::no_match(DetectorType::Ocr))
                }
            }
            Err(e) => {
                log::warn!("OCR failed: {}", e);
                Ok(DetectionResult::no_match(DetectorType::Ocr))
            }
        }
    }

    fn detector_type(&self) -> DetectorType {
        DetectorType::Ocr
    }
}

/// Factory function to create detector from config
pub fn create_detector(
    detector_type: &str,
    template_config: Option<&TemplateConfig>,
    color_config: Option<&ColorConfig>,
    ocr_config: Option<&OcrConfig>,
    region: Option<&RegionConfig>,
    base_path: &Path,
) -> Result<Box<dyn Detector>> {
    match detector_type {
        "template" => {
            let config = template_config.ok_or_else(|| anyhow!("Template config required"))?;
            Ok(Box::new(TemplateDetector::new(config, base_path, region)?))
        }
        "color" => {
            let config = color_config.ok_or_else(|| anyhow!("Color config required"))?;
            Ok(Box::new(ColorDetector::new(config, region)))
        }
        "ocr" => {
            let config = ocr_config.ok_or_else(|| anyhow!("OCR config required"))?;
            Ok(Box::new(OcrDetector::new(config, region)))
        }
        _ => Err(anyhow!("Unknown detector type: {}", detector_type)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;

    #[test]
    fn test_color_detector() {
        let config = ColorConfig {
            target: [255, 0, 0], // Red
            tolerance: 10,
            min_match_percent: 50.0,
            detect_presence: true,
        };

        let detector = ColorDetector::new(&config, None);

        // Create a red frame
        let mut pixels = vec![0u8; 100 * 100 * 4];
        for chunk in pixels.chunks_exact_mut(4) {
            chunk[0] = 255; // R
            chunk[1] = 0; // G
            chunk[2] = 0; // B
            chunk[3] = 255; // A
        }

        let frame = FrameData::new(pixels, 100, 100);
        let result = detector.detect(&frame).unwrap();

        assert!(result.matched);
        assert!(result.confidence > 0.9);
    }

    #[test]
    fn test_template_ncc() {
        // Create a simple test pattern
        let mut img = GrayImage::new(100, 100);
        for y in 40..60 {
            for x in 40..60 {
                img.put_pixel(x, y, Luma([255]));
            }
        }

        let mut template = GrayImage::new(20, 20);
        for y in 0..20 {
            for x in 0..20 {
                template.put_pixel(x, y, Luma([255]));
            }
        }

        let (corr, x, y) = template_match_ncc(&img, &template);

        assert!(corr > 0.9, "Correlation should be high: {}", corr);
        assert_eq!(x, 40, "X should be 40");
        assert_eq!(y, 40, "Y should be 40");
    }
}
