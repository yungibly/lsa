use crate::cache::{Cache, Key};
use image::{
    DynamicImage, ImageDecoder, ImageReader, Limits, Rgba, RgbaImage, metadata::Orientation,
};
use std::{
    fs::{self, OpenOptions},
    io::{self, BufRead, BufReader, Read, Seek, SeekFrom},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};

pub const INPUT_LIMIT: u64 = 32 * 1024 * 1024;
pub const PIXEL_LIMIT: u64 = 16_000_000;
pub const ALLOC_LIMIT: u64 = 64 * 1024 * 1024;
pub const OUTPUT_LIMIT: usize = 128 * 1024 * 1024;
pub const PLACEMENT_LIMIT: usize = 4096;

pub struct Budget {
    pub attempts_left: usize,
    pub bytes_left: usize,
    pub placements_left: usize,
}
impl Budget {
    pub fn new(attempts: usize) -> Self {
        Self {
            attempts_left: attempts,
            bytes_left: OUTPUT_LIMIT,
            placements_left: PLACEMENT_LIMIT,
        }
    }
    pub fn can_draw(&self, bytes: usize) -> bool {
        self.placements_left > 0 && bytes <= self.bytes_left
    }
    pub fn placed(&mut self, bytes: usize) {
        self.bytes_left -= bytes;
        self.placements_left -= 1;
    }
    pub fn begin(&mut self, bytes: usize) -> bool {
        if self.attempts_left == 0 || !self.can_draw(bytes) {
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

pub fn load(
    path: &Path,
    width: u32,
    height: u32,
    cache: &mut Cache<'_>,
) -> Result<RgbaImage, Box<dyn std::error::Error>> {
    if width == 0 || height == 0 || width > 320 || height > 240 {
        return Err(invalid("invalid thumbnail size").into());
    }
    // Precheck avoids opening known devices; O_NONBLOCK prevents a replacement
    // FIFO (including a symlink target) from hanging open. Verify the open fd too.
    if !fs::metadata(path)?.is_file() {
        return Err(invalid("not a regular file").into());
    }
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() > INPUT_LIMIT {
        return Err(invalid("not a bounded regular file").into());
    }
    // Open/validate the source even on hits: unreadable, replaced, oversized, or
    // special files must not acquire a preview merely because storage is warm.
    let key = cache.enabled().then(|| Key::new(&meta, width, height));
    if let Some(key) = &key
        && let Some(image) = cache.get(key)
    {
        if *key != Key::new(&file.metadata()?, width, height) {
            return Err(invalid("source changed during cache lookup").into());
        }
        return Ok(image);
    }
    let source = Bounded {
        reader: &mut file,
        len: meta.len(),
        position: 0,
    };
    let image = decode_reader(BufReader::new(source), width, height)?;
    if let Some(key) = key
        && key == Key::new(&file.metadata()?, width, height)
    {
        cache.put(&key, &image);
    }
    Ok(image)
}

/// Snapshot-length reader: a growing source cannot make a decoder read or
/// allocate beyond the validated input budget. Seeking remains bounded too.
struct Bounded<R> {
    reader: R,
    len: u64,
    position: u64,
}
impl<R: Read> Read for Bounded<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = buf.len().min((self.len - self.position) as usize);
        let read = self.reader.read(&mut buf[..count])?;
        self.position += read as u64;
        Ok(read)
    }
}
impl<R: Seek> Seek for Bounded<R> {
    fn seek(&mut self, offset: SeekFrom) -> io::Result<u64> {
        let position = match offset {
            SeekFrom::Start(n) => i128::from(n),
            SeekFrom::End(n) => i128::from(self.len) + i128::from(n),
            SeekFrom::Current(n) => i128::from(self.position) + i128::from(n),
        };
        if !(0..=i128::from(self.len)).contains(&position) {
            return Err(invalid("seek exceeds source bounds"));
        }
        self.reader.seek(SeekFrom::Start(position as u64))?;
        self.position = position as u64;
        Ok(self.position)
    }
}

fn decode_reader(
    reader: impl BufRead + Seek,
    width: u32,
    height: u32,
) -> Result<RgbaImage, Box<dyn std::error::Error>> {
    if width == 0 || height == 0 || width > 320 || height > 240 {
        return Err(invalid("invalid thumbnail size").into());
    }
    let mut reader = ImageReader::new(reader).with_guessed_format()?;
    if reader.format().is_none() {
        let mut bytes = Vec::new();
        reader
            .into_inner()
            .take(crate::svg::INPUT_LIMIT as u64 + 1)
            .read_to_end(&mut bytes)?;
        return crate::svg::decode(&bytes, width, height);
    }
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
    let image = DynamicImage::from_decoder(decoder)?;
    // Rotate only the thumbnail, avoiding a second full-resolution allocation
    // and a full-image rotation pass for portrait JPEGs.
    let swap = matches!(
        orientation,
        Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    );
    let mut thumb = if swap {
        image.thumbnail(height, width)
    } else {
        image.thumbnail(width, height)
    };
    thumb.apply_orientation(orientation);
    Ok(canvas(&thumb.into_rgba8(), width, height))
}

#[cfg(test)]
fn fit(image: &DynamicImage, width: u32, height: u32) -> RgbaImage {
    let thumb = image.thumbnail(width, height).into_rgba8();
    canvas(&thumb, width, height)
}

pub(crate) fn canvas(thumb: &RgbaImage, width: u32, height: u32) -> RgbaImage {
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
    use std::io::Cursor;
    fn decode(
        bytes: &[u8],
        width: u32,
        height: u32,
    ) -> Result<RgbaImage, Box<dyn std::error::Error>> {
        decode_reader(Cursor::new(bytes), width, height)
    }
    #[test]
    fn all_enabled_formats_decode() {
        for format in [
            image::ImageFormat::Png,
            image::ImageFormat::Jpeg,
            image::ImageFormat::Gif,
            image::ImageFormat::WebP,
            image::ImageFormat::Bmp,
            image::ImageFormat::Ico,
        ] {
            let img = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
                12,
                8,
                image::Rgb([255, 0, 0]),
            ));
            let mut data = Cursor::new(Vec::new());
            if format == image::ImageFormat::Ico {
                DynamicImage::ImageRgba8(img.into_rgba8())
                    .write_to(&mut data, format)
                    .unwrap();
            } else {
                img.write_to(&mut data, format).unwrap();
            }
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
        let mut b = Budget::new(4096);
        for _ in 0..PLACEMENT_LIMIT {
            assert!(b.can_draw(1));
            b.placed(1);
        }
        assert!(!b.can_draw(1));
        // Even maximum-size thumbnails fit 256 attempts at the default cap.
        let bytes = crate::kitty::byte_len(320, 240, 56, 12);
        assert!(bytes * 256 < OUTPUT_LIMIT);
    }
    #[test]
    fn source_reader_cannot_seek_or_read_past_validated_length() {
        let mut source = Bounded {
            reader: Cursor::new(b"123456789"),
            len: 5,
            position: 0,
        };
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"12345");
        assert!(source.seek(SeekFrom::Start(6)).is_err());
        assert!(source.seek(SeekFrom::End(-6)).is_err());
        source.seek(SeekFrom::End(-2)).unwrap();
        bytes.clear();
        source.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"45");
    }
    #[test]
    fn every_exif_orientation_preserves_asymmetric_thumbnail_content() {
        let source = DynamicImage::ImageRgb8(image::RgbImage::from_fn(120, 80, |x, y| {
            image::Rgb([
                if x < 60 { 240 } else { 10 },
                if y < 40 { 240 } else { 10 },
                0,
            ])
        }));
        let mut data = Cursor::new(Vec::new());
        source
            .write_to(&mut data, image::ImageFormat::Jpeg)
            .unwrap();
        for value in 1..=8 {
            let mut exif =
                b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x01\0\0\0\0\0\0\0"
                    .to_vec();
            exif[24] = value;
            let mut jpeg = vec![0xff, 0xd8, 0xff, 0xe1];
            jpeg.extend(((exif.len() + 2) as u16).to_be_bytes());
            jpeg.extend(exif);
            jpeg.extend(&data.get_ref()[2..]);
            let mut full = image::load_from_memory(&jpeg).unwrap();
            full.apply_orientation(Orientation::from_exif(value).unwrap());
            let expected = fit(&full, 24, 16);
            let actual = decode(&jpeg, 24, 16).unwrap();
            // Area sampling reverses fractional edge bins when rotation moves
            // after resizing. Check coverage and overall color error, allowing
            // that small edge difference rather than requiring identical bins.
            assert!(
                actual
                    .pixels()
                    .zip(expected.pixels())
                    .all(|(a, e)| a[3] == e[3])
            );
            let error: u32 = actual
                .as_raw()
                .iter()
                .zip(expected.as_raw())
                .map(|(a, e)| u32::from(a.abs_diff(*e)))
                .sum();
            assert!(
                error < 5 * actual.as_raw().len() as u32,
                "orientation {value}: {error}"
            );
        }
    }
}
