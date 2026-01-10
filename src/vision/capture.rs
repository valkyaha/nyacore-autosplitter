//! Video capture abstraction for different sources
//!
//! Supports:
//! - Window capture (capture card window, OBS preview)
//! - Screen region capture
//! - Future: OBS WebSocket, NDI

use anyhow::{anyhow, Result};
use image::{DynamicImage, RgbaImage};
use std::sync::Arc;

/// Raw frame data from capture source
#[derive(Clone)]
pub struct FrameData {
    /// RGBA pixel data
    pub pixels: Arc<Vec<u8>>,
    /// Frame width
    pub width: u32,
    /// Frame height
    pub height: u32,
    /// Timestamp in milliseconds
    pub timestamp: u64,
}

impl FrameData {
    /// Create new frame data
    pub fn new(pixels: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            pixels: Arc::new(pixels),
            width,
            height,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    /// Convert to image crate's DynamicImage
    pub fn to_image(&self) -> Option<DynamicImage> {
        RgbaImage::from_raw(self.width, self.height, (*self.pixels).clone())
            .map(DynamicImage::ImageRgba8)
    }

    /// Extract a region from the frame
    pub fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Option<FrameData> {
        if x + width > self.width || y + height > self.height {
            return None;
        }

        let mut cropped = Vec::with_capacity((width * height * 4) as usize);
        for row in y..(y + height) {
            let start = ((row * self.width + x) * 4) as usize;
            let end = start + (width * 4) as usize;
            cropped.extend_from_slice(&self.pixels[start..end]);
        }

        Some(FrameData::new(cropped, width, height))
    }

    /// Scale down the frame for faster processing
    pub fn scale(&self, factor: f32) -> FrameData {
        if let Some(img) = self.to_image() {
            let new_width = (self.width as f32 * factor) as u32;
            let new_height = (self.height as f32 * factor) as u32;
            let scaled = img.resize_exact(
                new_width,
                new_height,
                image::imageops::FilterType::Nearest,
            );
            let rgba = scaled.to_rgba8();
            FrameData::new(rgba.into_raw(), new_width, new_height)
        } else {
            self.clone()
        }
    }
}

/// Trait for capture sources
pub trait CaptureSource: Send + Sync {
    /// Capture a single frame
    fn capture(&mut self) -> Result<FrameData>;

    /// Check if source is still valid/available
    fn is_available(&self) -> bool;

    /// Get source dimensions
    fn dimensions(&self) -> (u32, u32);
}

/// Window capture source (Windows-specific)
#[cfg(target_os = "windows")]
pub mod window_capture {
    use super::*;
    use std::sync::atomic::{AtomicIsize, Ordering};
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        SRCCOPY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        FindWindowW, GetClientRect, GetWindowRect, IsWindow,
    };

    /// Thread-safe wrapper for HWND
    pub struct WindowCapture {
        // Store as atomic isize for thread safety
        hwnd_ptr: AtomicIsize,
        width: u32,
        height: u32,
        window_title: String,
    }

    // Safety: HWND is just a pointer that can be safely sent between threads
    // The actual window operations are thread-safe in Windows
    unsafe impl Send for WindowCapture {}
    unsafe impl Sync for WindowCapture {}

    impl WindowCapture {
        /// Create capture from window title (partial match)
        pub fn from_title(title: &str) -> Result<Self> {
            let hwnd = find_window_by_title(title)?;
            let (width, height) = get_window_size(hwnd)?;
            Ok(Self {
                hwnd_ptr: AtomicIsize::new(hwnd.0 as isize),
                width,
                height,
                window_title: title.to_string(),
            })
        }

        fn get_hwnd(&self) -> HWND {
            HWND(self.hwnd_ptr.load(Ordering::SeqCst) as *mut _)
        }

        /// Refresh window handle if needed
        pub fn refresh(&mut self) -> Result<()> {
            let hwnd = find_window_by_title(&self.window_title)?;
            let (w, h) = get_window_size(hwnd)?;
            self.hwnd_ptr.store(hwnd.0 as isize, Ordering::SeqCst);
            self.width = w;
            self.height = h;
            Ok(())
        }
    }

    impl CaptureSource for WindowCapture {
        fn capture(&mut self) -> Result<FrameData> {
            let hwnd = self.get_hwnd();

            unsafe {
                // Get device contexts
                let hdc_window = GetDC(hwnd);
                if hdc_window.is_invalid() {
                    return Err(anyhow!("Failed to get window DC"));
                }

                let hdc_mem = CreateCompatibleDC(hdc_window);
                if hdc_mem.is_invalid() {
                    ReleaseDC(hwnd, hdc_window);
                    return Err(anyhow!("Failed to create compatible DC"));
                }

                // Create bitmap
                let hbitmap =
                    CreateCompatibleBitmap(hdc_window, self.width as i32, self.height as i32);
                if hbitmap.is_invalid() {
                    let _ = DeleteDC(hdc_mem);
                    ReleaseDC(hwnd, hdc_window);
                    return Err(anyhow!("Failed to create bitmap"));
                }

                let old_bitmap = SelectObject(hdc_mem, hbitmap);

                // Copy window content
                let result = BitBlt(
                    hdc_mem,
                    0,
                    0,
                    self.width as i32,
                    self.height as i32,
                    hdc_window,
                    0,
                    0,
                    SRCCOPY,
                );

                if result.is_err() {
                    SelectObject(hdc_mem, old_bitmap);
                    let _ = DeleteObject(hbitmap);
                    let _ = DeleteDC(hdc_mem);
                    ReleaseDC(hwnd, hdc_window);
                    return Err(anyhow!("BitBlt failed"));
                }

                // Get pixel data
                let mut bmi = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: self.width as i32,
                        biHeight: -(self.height as i32), // Top-down
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB.0,
                        ..Default::default()
                    },
                    ..Default::default()
                };

                let mut pixels = vec![0u8; (self.width * self.height * 4) as usize];

                GetDIBits(
                    hdc_mem,
                    hbitmap,
                    0,
                    self.height,
                    Some(pixels.as_mut_ptr() as *mut _),
                    &mut bmi,
                    DIB_RGB_COLORS,
                );

                // Convert BGRA to RGBA
                for chunk in pixels.chunks_exact_mut(4) {
                    chunk.swap(0, 2);
                }

                // Cleanup
                SelectObject(hdc_mem, old_bitmap);
                let _ = DeleteObject(hbitmap);
                let _ = DeleteDC(hdc_mem);
                ReleaseDC(hwnd, hdc_window);

                Ok(FrameData::new(pixels, self.width, self.height))
            }
        }

        fn is_available(&self) -> bool {
            unsafe { IsWindow(self.get_hwnd()).as_bool() }
        }

        fn dimensions(&self) -> (u32, u32) {
            (self.width, self.height)
        }
    }

    fn find_window_by_title(title: &str) -> Result<HWND> {
        // Try exact match first
        let wide_title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        let hwnd = unsafe { FindWindowW(None, windows::core::PCWSTR(wide_title.as_ptr())) };

        match hwnd {
            Ok(h) if !h.is_invalid() => return Ok(h),
            _ => {}
        }

        Err(anyhow!("Window not found: {}", title))
    }

    fn get_window_size(hwnd: HWND) -> Result<(u32, u32)> {
        unsafe {
            let mut rect = RECT::default();
            if GetClientRect(hwnd, &mut rect).is_ok() {
                Ok((
                    (rect.right - rect.left) as u32,
                    (rect.bottom - rect.top) as u32,
                ))
            } else {
                // Fall back to window rect
                let mut rect = RECT::default();
                if GetWindowRect(hwnd, &mut rect).is_ok() {
                    Ok((
                        (rect.right - rect.left) as u32,
                        (rect.bottom - rect.top) as u32,
                    ))
                } else {
                    Err(anyhow!("Failed to get window size"))
                }
            }
        }
    }
}

/// File-based capture for testing
pub struct FileCapture {
    image: DynamicImage,
}

impl FileCapture {
    pub fn new(path: &std::path::Path) -> Result<Self> {
        let image = image::open(path)?;
        Ok(Self { image })
    }
}

impl CaptureSource for FileCapture {
    fn capture(&mut self) -> Result<FrameData> {
        let rgba = self.image.to_rgba8();
        Ok(FrameData::new(
            rgba.clone().into_raw(),
            rgba.width(),
            rgba.height(),
        ))
    }

    fn is_available(&self) -> bool {
        true
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.image.width(), self.image.height())
    }
}

/// Factory function to create appropriate capture source
#[allow(unused_variables)]
pub fn create_capture_source(
    source_type: &str,
    window_title: Option<&str>,
    _obs_source: Option<&str>,
) -> Result<Box<dyn CaptureSource>> {
    match source_type {
        #[cfg(target_os = "windows")]
        "window" => {
            let title = window_title.ok_or_else(|| anyhow!("Window title required"))?;
            Ok(Box::new(window_capture::WindowCapture::from_title(title)?))
        }
        #[cfg(not(target_os = "windows"))]
        "window" => Err(anyhow!("Window capture not supported on this platform")),
        "obs" => {
            // TODO: Implement OBS WebSocket capture
            Err(anyhow!("OBS capture not yet implemented"))
        }
        _ => Err(anyhow!("Unknown capture source type: {}", source_type)),
    }
}

/// Video file capture for testing (requires video-test feature)
#[cfg(feature = "video-test")]
pub mod video_capture {
    use super::*;
    use std::path::Path;
    use std::sync::Mutex;

    /// Video file capture source using FFmpeg
    pub struct VideoCapture {
        decoder: Mutex<VideoDecoder>,
        width: u32,
        height: u32,
        loop_video: bool,
        playback_speed: f32,
        last_frame_time: Mutex<std::time::Instant>,
        frame_duration: std::time::Duration,
    }

    struct VideoDecoder {
        input_ctx: ffmpeg_next::format::context::Input,
        decoder: ffmpeg_next::decoder::Video,
        video_stream_index: usize,
        scaler: Option<ffmpeg_next::software::scaling::Context>,
        width: u32,
        height: u32,
        frame_rate: f64,
        current_frame: Option<FrameData>,
    }

    impl VideoCapture {
        /// Open a video file for capture
        pub fn open(path: &Path, loop_video: bool, playback_speed: f32) -> Result<Self> {
            ffmpeg_next::init().map_err(|e| anyhow!("FFmpeg init failed: {}", e))?;

            let input_ctx = ffmpeg_next::format::input(&path)
                .map_err(|e| anyhow!("Failed to open video: {}", e))?;

            // Find video stream
            let video_stream = input_ctx
                .streams()
                .best(ffmpeg_next::media::Type::Video)
                .ok_or_else(|| anyhow!("No video stream found"))?;

            let video_stream_index = video_stream.index();

            // Get decoder
            let context_decoder = ffmpeg_next::codec::context::Context::from_parameters(
                video_stream.parameters(),
            )?;
            let decoder = context_decoder.decoder().video()?;

            let width = decoder.width();
            let height = decoder.height();

            // Calculate frame rate
            let frame_rate = video_stream.avg_frame_rate();
            let fps = if frame_rate.denominator() > 0 {
                frame_rate.numerator() as f64 / frame_rate.denominator() as f64
            } else {
                30.0 // Default
            };

            let frame_duration = std::time::Duration::from_secs_f64(1.0 / (fps * playback_speed as f64));

            log::info!(
                "Opened video: {}x{} @ {:.2} fps (playback: {:.1}x)",
                width, height, fps, playback_speed
            );

            let video_decoder = VideoDecoder {
                input_ctx,
                decoder,
                video_stream_index,
                scaler: None,
                width,
                height,
                frame_rate: fps,
                current_frame: None,
            };

            Ok(Self {
                decoder: Mutex::new(video_decoder),
                width,
                height,
                loop_video,
                playback_speed,
                last_frame_time: Mutex::new(std::time::Instant::now()),
                frame_duration,
            })
        }

        fn decode_next_frame(decoder: &mut VideoDecoder) -> Result<Option<FrameData>> {
            let mut frame = ffmpeg_next::frame::Video::empty();

            loop {
                // Try to receive a decoded frame
                match decoder.decoder.receive_frame(&mut frame) {
                    Ok(_) => {
                        // Convert to RGBA
                        let scaler = decoder.scaler.get_or_insert_with(|| {
                            ffmpeg_next::software::scaling::Context::get(
                                decoder.decoder.format(),
                                decoder.width,
                                decoder.height,
                                ffmpeg_next::format::Pixel::RGBA,
                                decoder.width,
                                decoder.height,
                                ffmpeg_next::software::scaling::Flags::BILINEAR,
                            )
                            .expect("Failed to create scaler")
                        });

                        let mut rgb_frame = ffmpeg_next::frame::Video::empty();
                        scaler.run(&frame, &mut rgb_frame)?;

                        let data = rgb_frame.data(0);
                        let stride = rgb_frame.stride(0);
                        let width = decoder.width as usize;
                        let height = decoder.height as usize;

                        // Copy pixel data (handle stride)
                        let mut pixels = Vec::with_capacity(width * height * 4);
                        for y in 0..height {
                            let row_start = y * stride;
                            let row_end = row_start + width * 4;
                            pixels.extend_from_slice(&data[row_start..row_end]);
                        }

                        return Ok(Some(FrameData::new(pixels, decoder.width, decoder.height)));
                    }
                    Err(ffmpeg_next::Error::Other { errno: ffmpeg_next::error::EAGAIN }) => {
                        // Need more packets
                    }
                    Err(ffmpeg_next::Error::Eof) => {
                        return Ok(None);
                    }
                    Err(e) => {
                        return Err(anyhow!("Decode error: {}", e));
                    }
                }

                // Read next packet
                let mut packet_found = false;
                for (stream, packet) in decoder.input_ctx.packets() {
                    if stream.index() == decoder.video_stream_index {
                        decoder.decoder.send_packet(&packet)?;
                        packet_found = true;
                        break;
                    }
                }

                if !packet_found {
                    // End of file
                    decoder.decoder.send_eof()?;
                }
            }
        }
    }

    impl CaptureSource for VideoCapture {
        fn capture(&mut self) -> Result<FrameData> {
            let mut decoder = self.decoder.lock().unwrap();
            let mut last_time = self.last_frame_time.lock().unwrap();

            // Wait for frame timing
            let elapsed = last_time.elapsed();
            if elapsed < self.frame_duration {
                std::thread::sleep(self.frame_duration - elapsed);
            }
            *last_time = std::time::Instant::now();

            // Decode next frame
            match Self::decode_next_frame(&mut decoder)? {
                Some(frame) => {
                    decoder.current_frame = Some(frame.clone());
                    Ok(frame)
                }
                None => {
                    if self.loop_video {
                        // Seek back to beginning
                        log::info!("Video ended, looping...");
                        decoder.input_ctx.seek(0, ..)?;
                        decoder.decoder.flush();

                        // Decode first frame
                        match Self::decode_next_frame(&mut decoder)? {
                            Some(frame) => {
                                decoder.current_frame = Some(frame.clone());
                                Ok(frame)
                            }
                            None => Err(anyhow!("Failed to decode after seek")),
                        }
                    } else {
                        // Return last frame or error
                        decoder
                            .current_frame
                            .clone()
                            .ok_or_else(|| anyhow!("Video ended"))
                    }
                }
            }
        }

        fn is_available(&self) -> bool {
            true
        }

        fn dimensions(&self) -> (u32, u32) {
            (self.width, self.height)
        }
    }
}

/// Directory of frames capture for testing (no external dependencies)
pub struct FrameSequenceCapture {
    frames: Vec<DynamicImage>,
    current_index: usize,
    loop_sequence: bool,
}

impl FrameSequenceCapture {
    /// Load frames from a directory (sorted by filename)
    pub fn from_directory(dir: &std::path::Path, loop_sequence: bool) -> Result<Self> {
        let mut paths: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .map(|ext| {
                        let ext = ext.to_string_lossy().to_lowercase();
                        ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "bmp"
                    })
                    .unwrap_or(false)
            })
            .collect();

        paths.sort();

        if paths.is_empty() {
            return Err(anyhow!("No image files found in directory"));
        }

        let frames: Vec<_> = paths
            .iter()
            .filter_map(|p| image::open(p).ok())
            .collect();

        log::info!("Loaded {} frames from {:?}", frames.len(), dir);

        Ok(Self {
            frames,
            current_index: 0,
            loop_sequence,
        })
    }
}

impl CaptureSource for FrameSequenceCapture {
    fn capture(&mut self) -> Result<FrameData> {
        if self.frames.is_empty() {
            return Err(anyhow!("No frames loaded"));
        }

        let frame = &self.frames[self.current_index];
        let rgba = frame.to_rgba8();
        let data = FrameData::new(rgba.clone().into_raw(), rgba.width(), rgba.height());

        // Advance to next frame
        self.current_index += 1;
        if self.current_index >= self.frames.len() {
            if self.loop_sequence {
                self.current_index = 0;
            } else {
                self.current_index = self.frames.len() - 1; // Stay on last frame
            }
        }

        Ok(data)
    }

    fn is_available(&self) -> bool {
        !self.frames.is_empty()
    }

    fn dimensions(&self) -> (u32, u32) {
        if let Some(first) = self.frames.first() {
            (first.width(), first.height())
        } else {
            (0, 0)
        }
    }
}
