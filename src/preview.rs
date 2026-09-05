use image::{DynamicImage, ImageDecoder, ImageReader, Limits, Rgba, RgbaImage};
use std::{
    fs::{self, OpenOptions},
    io::{self, Cursor, Read},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};

pub const INPUT_LIMIT: u64 = 32 * 1024 * 1024;
pub const PIXEL_LIMIT: u64 = 16_000_000;
pub const ALLOC_LIMIT: u64 = 64 * 1024 * 1024;
pub const OUTPUT_LIMIT: usize = 8 * 1024 * 1024;

pub struct Budget {
    pub attempts_left: usize,
    pub bytes_left: usize,
    pub shown: usize,
    pub failed: usize,
    pub limited: usize,
}
impl Budget {
    pub fn new(attempts: usize) -> Self {
        Self {
            attempts_left: attempts,
            bytes_left: OUTPUT_LIMIT,
            shown: 0,
            failed: 0,
            limited: 0,
        }
    }
    pub fn begin(&mut self, bytes: usize) -> bool {
        if self.attempts_left == 0 || bytes > self.bytes_left {
            self.limited += 1;
            false
        } else {
            self.attempts_left -= 1;
            true
        }
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub fn load(path: &Path, width: u32, height: u32) -> Result<RgbaImage, Box<dyn std::error::Error>> {
    // Precheck avoids opening known devices; O_NONBLOCK prevents a replacement
    // FIFO (including a symlink target) from hanging open. Verify the open fd too.
    if !fs::metadata(path)?.is_file() {
        return Err(invalid("not a regular file").into());
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() > INPUT_LIMIT {
        return Err(invalid("not a bounded regular file").into());
    }
    let mut bytes = Vec::with_capacity(meta.len() as usize);
    file.take(INPUT_LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > INPUT_LIMIT {
        return Err(invalid("source exceeds input limit").into());
    }
    decode(&bytes, width, height)
}

fn decode(bytes: &[u8], width: u32, height: u32) -> Result<RgbaImage, Box<dyn std::error::Error>> {
    if width == 0 || height == 0 || width > 320 || height > 240 {
        return Err(invalid("invalid thumbnail size").into());
    }
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(16_384);
    limits.max_image_height = Some(16_384);
    limits.max_alloc = Some(ALLOC_LIMIT);
    reader.limits(limits);
    let mut decoder = reader.into_decoder()?;
    let (w, h) = decoder.dimensions();
    if u64::from(w) * u64::from(h) > PIXEL_LIMIT || decoder.total_bytes() > ALLOC_LIMIT {
        return Err(invalid("image exceeds decode limits").into());
    }
    let orientation = decoder.orientation()?;
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    Ok(fit(&image, width, height))
}

fn fit(image: &DynamicImage, width: u32, height: u32) -> RgbaImage {
    let thumb = image.thumbnail(width, height).into_rgba8();
    let x = (width - thumb.width()) / 2;
    let y = (height - thumb.height()) / 2;
    let mut canvas = RgbaImage::new(width, height);
    // Neutral checker only beneath the image; letterboxing stays transparent.
    for (tx, ty, pixel) in thumb.enumerate_pixels() {
        let alpha = u32::from(pixel[3]);
        let bg = if (tx / 8 + ty / 8) % 2 == 0 { 176 } else { 216 };
        let mut color = [0, 0, 0, 255];
        for i in 0..3 {
            color[i] = ((u32::from(pixel[i]) * alpha + bg * (255 - alpha) + 127) / 255) as u8;
        }
        canvas.put_pixel(x + tx, y + ty, Rgba(color));
    }
    canvas
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_enabled_formats_decode() {
        for format in [
            image::ImageFormat::Png,
            image::ImageFormat::Jpeg,
            image::ImageFormat::Gif,
            image::ImageFormat::WebP,
            image::ImageFormat::Bmp,
        ] {
            let img = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
                12,
                8,
                image::Rgb([255, 0, 0]),
            ));
            let mut data = Cursor::new(Vec::new());
            img.write_to(&mut data, format).unwrap();
            let thumb = decode(data.get_ref(), 40, 40).unwrap();
            assert_eq!(thumb.dimensions(), (40, 40));
            assert!(thumb.get_pixel(20, 20)[0] > 240, "{format:?}");
        }
    }
    #[test]
    fn aspect_and_transparency() {
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(40, 20, Rgba([255, 0, 0, 0])));
        let result = fit(&img, 40, 40);
        assert_eq!(result.get_pixel(0, 0)[3], 0);
        assert_eq!(result.get_pixel(0, 10), &Rgba([176, 176, 176, 255]));
        assert_eq!(result.get_pixel(8, 10), &Rgba([216, 216, 216, 255]));
        assert_eq!(result.get_pixel(0, 30)[3], 0);
    }
    #[test]
    fn rejects_corruption_and_excessive_dimensions() {
        assert!(decode(b"not an image", 40, 40).is_err());
        let img = DynamicImage::new_rgb8(16_385, 1);
        let mut bytes = Cursor::new(Vec::new());
        img.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        assert!(decode(bytes.get_ref(), 40, 40).is_err());
    }
    #[test]
    fn pixel_product_is_limited_before_decode() {
        let img = DynamicImage::new_luma8(4_001, 4_000);
        let mut bytes = Cursor::new(Vec::new());
        img.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        assert!(
            decode(bytes.get_ref(), 40, 40)
                .unwrap_err()
                .to_string()
                .contains("decode limits")
        );
    }
    #[test]
    fn applies_jpeg_exif_orientation() {
        let img =
            DynamicImage::ImageRgb8(image::RgbImage::from_pixel(40, 20, image::Rgb([255, 0, 0])));
        let mut bytes = Cursor::new(Vec::new());
        img.write_to(&mut bytes, image::ImageFormat::Jpeg).unwrap();
        // APP1 Exif: little-endian TIFF, one SHORT orientation tag, value 6.
        let exif = b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
        let mut jpeg = vec![0xff, 0xd8, 0xff, 0xe1];
        jpeg.extend(((exif.len() + 2) as u16).to_be_bytes());
        jpeg.extend(exif);
        jpeg.extend(&bytes.get_ref()[2..]);
        let result = decode(&jpeg, 40, 40).unwrap();
        assert_eq!(result.get_pixel(0, 20)[3], 0); // Rotated portrait, left margin.
        assert_eq!(result.get_pixel(20, 0)[3], 255);
        assert!(result.get_pixel(20, 20)[0] > 240);
    }
    #[test]
    fn gif_first_frame_uses_logical_canvas_and_ignores_later_frames() {
        let mut bytes = Vec::new();
        {
            let mut encoder =
                gif::Encoder::new(&mut bytes, 10, 10, &[255, 0, 0, 0, 0, 255]).unwrap();
            let frame = gif::Frame {
                left: 3,
                top: 4,
                width: 2,
                height: 2,
                buffer: std::borrow::Cow::Owned(vec![0; 4]),
                ..gif::Frame::default()
            };
            encoder.write_frame(&frame).unwrap();
            let second = gif::Frame {
                width: 10,
                height: 10,
                buffer: std::borrow::Cow::Owned(vec![1; 100]),
                ..gif::Frame::default()
            };
            encoder.write_frame(&second).unwrap();
        }
        let result = decode(&bytes, 10, 10).unwrap();
        assert_eq!(result.get_pixel(3, 4), &Rgba([255, 0, 0, 255]));
        assert_eq!(result.get_pixel(4, 5), &Rgba([255, 0, 0, 255]));
        assert_eq!(result.get_pixel(0, 0), &Rgba([176, 176, 176, 255]));
        assert_ne!(result.get_pixel(5, 5), &Rgba([0, 0, 255, 255]));
    }
    #[test]
    fn attempts_include_failures_and_output_is_bounded() {
        let mut b = Budget::new(2);
        assert!(b.begin(100));
        assert!(b.begin(100));
        assert!(!b.begin(100));
        let mut b = Budget::new(10);
        b.bytes_left = 99;
        assert!(!b.begin(100));
        assert_eq!(b.attempts_left, 10);
    }
}
