//! Frame encoding - Compress captured frames for UDP transport.
//!
//! Strategies:
//!   - RawRGBA: No compression, for gigabit LAN
//!   - Jpeg: Good compression ratio, acceptable CPU cost
//!   - Lz4: Fast compression, moderate ratio
//!   - H264: Best compression, via OpenH264 software encoder

use crate::protocol::FrameEncoding;
use crate::screen::capture::CapturedFrame;
use anyhow::Result;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use openh264::formats::YUVSource;

/// Encode a captured frame into the specified format
pub fn encode_frame(frame: &CapturedFrame, encoding: FrameEncoding) -> Result<Vec<u8>> {
    match encoding {
        FrameEncoding::RawRgba => encode_raw(frame),
        FrameEncoding::Jpeg => encode_jpeg(frame),
        FrameEncoding::Lz4 => encode_lz4(frame),
        FrameEncoding::H264 => encode_h264(frame),
    }
}

fn encode_raw(frame: &CapturedFrame) -> Result<Vec<u8>> {
    Ok(frame.data.clone())
}

fn encode_jpeg(frame: &CapturedFrame) -> Result<Vec<u8>> {
    let img = image::RgbaImage::from_raw(frame.width, frame.height, frame.data.clone())
        .ok_or_else(|| anyhow::anyhow!("Failed to create image from frame data"))?;

    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Jpeg)?;
    Ok(buf.into_inner())
}

fn encode_lz4(frame: &CapturedFrame) -> Result<Vec<u8>> {
    Ok(lz4_flex::compress_prepend_size(&frame.data))
}

// ─── H.264 Encoding (via OpenH264) ─────────────────────────────────

static H264_ENCODER: Lazy<Mutex<Option<(openh264::encoder::Encoder, u32, u32)>>> =
    Lazy::new(|| Mutex::new(None));

fn encode_h264(frame: &CapturedFrame) -> Result<Vec<u8>> {
    let w = frame.width as usize;
    let h = frame.height as usize;

    // Ensure even dimensions (required by H.264)
    let enc_w = (w + 1) & !1;
    let enc_h = (h + 1) & !1;

    let yuv_data = rgba_to_i420(&frame.data, w, h, enc_w, enc_h);

    let mut guard = H264_ENCODER.lock().unwrap();
    let needs_recreate = guard.as_ref().map_or(true, |(_, ew, eh)| {
        *ew != enc_w as u32 || *eh != enc_h as u32
    });

    if needs_recreate {
        // Create new encoder
        let config = openh264::encoder::EncoderConfig::new();
        let encoder = openh264::encoder::Encoder::with_api_config(
            openh264::OpenH264API::from_source(),
            config,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create H264 encoder: {}", e))?;
        *guard = Some((encoder, enc_w as u32, enc_h as u32));
    }

    let (encoder, _, _) = guard.as_mut().unwrap();

    let yuv_buffer = openh264::formats::YUVBuffer::from_vec(yuv_data, enc_w, enc_h);

    let bitstream = encoder
        .encode(&yuv_buffer)
        .map_err(|e| anyhow::anyhow!("H264 encode error: {}", e))?;

    // Serialize: [width:4][height:4][h264_data...]
    let mut output = Vec::with_capacity(8 + bitstream.to_vec().len());
    output.extend_from_slice(&(enc_w as u32).to_le_bytes());
    output.extend_from_slice(&(enc_h as u32).to_le_bytes());
    bitstream.write_vec(&mut output);

    Ok(output)
}

/// Convert RGBA to I420 YUV, optionally padding to even dimensions
fn rgba_to_i420(rgba: &[u8], width: usize, height: usize, enc_w: usize, enc_h: usize) -> Vec<u8> {
    let frame_size = enc_w * enc_h;
    let uv_size = frame_size / 4;
    let mut yuv = vec![0u8; frame_size + frame_size / 2];

    let (y_part, uv_part) = yuv.split_at_mut(frame_size);
    let (u_part, v_part) = uv_part.split_at_mut(uv_size);

    for y in 0..enc_h {
        for x in 0..enc_w {
            let src_x = if x < width { x } else { width - 1 };
            let src_y = if y < height { y } else { height - 1 };
            let px_idx = (src_y * width + src_x) * 4;

            let r = rgba[px_idx] as f32;
            let g = rgba[px_idx + 1] as f32;
            let b = rgba[px_idx + 2] as f32;

            // BT.601
            let y_val = (0.299 * r + 0.587 * g + 0.114 * b).clamp(0.0, 255.0) as u8;
            y_part[y * enc_w + x] = y_val;

            if y % 2 == 0 && x % 2 == 0 {
                let uv_idx = (y / 2) * (enc_w / 2) + (x / 2);
                u_part[uv_idx] = (-0.169 * r - 0.331 * g + 0.5 * b + 128.0).clamp(0.0, 255.0) as u8;
                v_part[uv_idx] = (0.5 * r - 0.419 * g - 0.081 * b + 128.0).clamp(0.0, 255.0) as u8;
            }
        }
    }

    yuv
}

// ─── Decoding ──────────────────────────────────────────────────────

/// Decode a frame for display
pub fn decode_frame(data: &[u8], encoding: FrameEncoding, width: u32, height: u32) -> Result<Vec<u8>> {
    match encoding {
        FrameEncoding::RawRgba => Ok(data.to_vec()),
        FrameEncoding::Jpeg => decode_jpeg(data),
        FrameEncoding::Lz4 => decode_lz4(data, width, height),
        FrameEncoding::H264 => decode_h264(data),
    }
}

fn decode_jpeg(data: &[u8]) -> Result<Vec<u8>> {
    let img = image::load_from_memory(data)?;
    let rgba = img.to_rgba8();
    Ok(rgba.into_raw())
}

fn decode_lz4(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let expected = (width * height * 4) as usize;
    let decompressed = lz4_flex::decompress_size_prepended(data)?;
    if decompressed.len() != expected {
        anyhow::bail!(
            "LZ4 decompressed size mismatch: expected {}, got {}",
            expected,
            decompressed.len()
        );
    }
    Ok(decompressed)
}

// ─── H.264 Decoding ────────────────────────────────────────────────

fn decode_h264(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < 8 {
        anyhow::bail!("H264 data too short");
    }

    let _width = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let _height = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let nal_data = &data[8..];

    let mut decoder = openh264::decoder::Decoder::with_api_config(
        openh264::OpenH264API::from_source(),
        openh264::decoder::DecoderConfig::new(),
    )
    .map_err(|e| anyhow::anyhow!("Failed to create H264 decoder: {}", e))?;

    let decoded = decoder
        .decode(nal_data)
        .map_err(|e| anyhow::anyhow!("H264 decode error: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("H264 decoder returned no frame"))?;

    // Read YUV planes from DecodedYUV
    let y = decoded.y();
    let u = decoded.u();
    let v = decoded.v();
    let (dec_w, dec_h) = decoded.dimensions();
    let (y_stride, uv_stride) = {
        let (ys, us, _) = decoded.strides();
        (ys, us)
    };

    let rgba = i420_to_rgba_strided(y, u, v, y_stride, uv_stride, dec_w, dec_h);
    Ok(rgba)
}

/// Convert I420 with strides to RGBA
fn i420_to_rgba_strided(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    y_stride: usize,
    uv_stride: usize,
    width: usize,
    height: usize,
) -> Vec<u8> {
    let mut rgba = vec![0u8; width * height * 4];

    for y_row in 0..height {
        for x in 0..width {
            let y_val = y_plane[y_row * y_stride + x] as f32;
            let uv_x = x / 2;
            let uv_y = y_row / 2;
            let u_val = u_plane[uv_y * uv_stride + uv_x] as f32 - 128.0;
            let v_val = v_plane[uv_y * uv_stride + uv_x] as f32 - 128.0;

            let r = (y_val + 1.402 * v_val).clamp(0.0, 255.0) as u8;
            let g = (y_val - 0.344 * u_val - 0.714 * v_val).clamp(0.0, 255.0) as u8;
            let b = (y_val + 1.772 * u_val).clamp(0.0, 255.0) as u8;

            let px_idx = (y_row * width + x) * 4;
            rgba[px_idx] = r;
            rgba[px_idx + 1] = g;
            rgba[px_idx + 2] = b;
            rgba[px_idx + 3] = 255;
        }
    }

    rgba
}
