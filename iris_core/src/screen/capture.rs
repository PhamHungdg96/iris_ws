//! Screen capture - Platform-specific implementations.
//!
//! Windows: DXGI Desktop Duplication API
//! macOS: CGDisplay (Core Graphics)
//! Android: Not directly capturable from native; Flutter provides the surface
//! Linux: Placeholder (X11 SHM or PipeWire)

use anyhow::Result;

/// Represents a captured screen frame as raw RGBA pixels
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    /// RGBA pixel data (width * height * 4 bytes)
    pub data: Vec<u8>,
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
}

/// Trait for platform-specific screen capture implementations
pub trait ScreenCapture: Send + Sync {
    /// Capture a single frame from the primary display
    fn capture_frame(&mut self) -> Result<CapturedFrame>;

    /// Get the dimensions of the primary display
    fn display_dimensions(&self) -> (u32, u32);
}

// ─── Platform-specific implementations ─────────────────────────────

// Inline conditional modules — each platform gets one implementation
// No need for `mod` declarations; Rust resolves via #[cfg] on `create_capture()`

/// Create a screen capturer for the current platform
pub fn create_capture() -> Result<Box<dyn ScreenCapture>> {
    #[cfg(target_os = "windows")]
    {
        windows_capture::WindowsCapture::new()
            .map(|c| Box::new(c) as Box<dyn ScreenCapture>)
    }

    #[cfg(target_os = "macos")]
    {
        macos_capture::MacOsCapture::new()
            .map(|c| Box::new(c) as Box<dyn ScreenCapture>)
    }

    #[cfg(target_os = "android")]
    {
        android_capture::AndroidCapture::new()
            .map(|c| Box::new(c) as Box<dyn ScreenCapture>)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "android")))]
    {
        fallback_capture::FallbackCapture::new()
            .map(|c| Box::new(c) as Box<dyn ScreenCapture>)
    }
}

// ─── Windows DXGI Desktop Duplication ──────────────────────────────

#[cfg(target_os = "windows")]
mod windows_capture {
    use super::*;
    use log::warn;

    pub struct WindowsCapture {
        width: u32,
        height: u32,
    }

    impl WindowsCapture {
        pub fn new() -> Result<Self> {
            // Note: Full DXGI Desktop Duplication requires COM initialization
            // and D3D11 device creation. This is a scaffold.
            // In production, use the `windows` crate:
            // - IDXGIOutputDuplication::AcquireNextFrame
            // - ID3D11Texture2D::Map for reading pixels
            warn!("Windows DXGI capture: using stub implementation");
            Ok(Self {
                width: 1920,
                height: 1080,
            })
        }
    }

    impl ScreenCapture for WindowsCapture {
        fn capture_frame(&mut self) -> Result<CapturedFrame> {
            // Stub: In production, capture via DXGI Desktop Duplication
            Ok(CapturedFrame {
                width: self.width,
                height: self.height,
                data: vec![0u8; (self.width * self.height * 4) as usize],
                timestamp_ms: std::time::UNIX_EPOCH
                    .elapsed()
                    .unwrap_or_default()
                    .as_millis() as u64,
            })
        }

        fn display_dimensions(&self) -> (u32, u32) {
            (self.width, self.height)
        }
    }
}

// ─── macOS CGDisplay ───────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod macos_capture {
    use super::*;
    use anyhow::Context;
    use core_graphics::display::CGDisplay;

    pub struct MacOsCapture {
        display: CGDisplay,
        width: u32,
        height: u32,
    }

    impl MacOsCapture {
        pub fn new() -> Result<Self> {
            let display = CGDisplay::main();
            let bounds = display.bounds();
            Ok(Self {
                display,
                width: bounds.size.width as u32,
                height: bounds.size.height as u32,
            })
        }
    }

    impl ScreenCapture for MacOsCapture {
        fn capture_frame(&mut self) -> Result<CapturedFrame> {
            // In production, use CGDisplayCreateImage or CGWindowListCreateImage
            // to capture screen contents
            let image = self
                .display
                .image()
                .context("Failed to capture CGDisplay image")?;

            let data = image
                .data()
                .to_vec();

            Ok(CapturedFrame {
                width: image.width() as u32,
                height: image.height() as u32,
                data,
                timestamp_ms: std::time::UNIX_EPOCH
                    .elapsed()
                    .unwrap_or_default()
                    .as_millis() as u64,
            })
        }

        fn display_dimensions(&self) -> (u32, u32) {
            (self.width, self.height)
        }
    }
}

// ─── Android (stub - Flutter provides frames) ──────────────────────

#[cfg(target_os = "android")]
mod android_capture {
    use super::*;

    pub struct AndroidCapture;

    impl AndroidCapture {
        pub fn new() -> Result<Self> {
            // On Android, screen capture requires MediaProjection API
            // which is handled from the Flutter/Java side via platform channels.
            // The native Rust code receives frames from Flutter via FFI.
            Ok(Self)
        }
    }

    impl ScreenCapture for AndroidCapture {
        fn capture_frame(&mut self) -> Result<CapturedFrame> {
            anyhow::bail!("Android capture is managed by Flutter platform code");
        }

        fn display_dimensions(&self) -> (u32, u32) {
            (0, 0)
        }
    }
}

// ─── Fallback (Linux etc.) ─────────────────────────────────────────

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "android")))]
mod fallback_capture {
    use super::*;
    use log::warn;

    pub struct FallbackCapture;

    impl FallbackCapture {
        pub fn new() -> Result<Self> {
            warn!("No native screen capture available on this platform");
            Ok(Self)
        }
    }

    impl ScreenCapture for FallbackCapture {
        fn capture_frame(&mut self) -> Result<CapturedFrame> {
            anyhow::bail!("Screen capture not implemented on this platform");
        }

        fn display_dimensions(&self) -> (u32, u32) {
            (1920, 1080)
        }
    }
}
