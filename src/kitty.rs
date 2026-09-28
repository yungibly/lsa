use base64::{Engine, engine::general_purpose::STANDARD};
use image::RgbaImage;
use std::io::{self, Write};

// 3 KiB of payload becomes exactly one 4 KiB protocol chunk.
const CHUNK: usize = 3072;
const CONTINUATION: &str = "\x1b_Gm=1,q=2;\x1b\\";

fn header(width: u32, height: u32, cols: usize, rows: usize, zlib: bool, more: bool) -> String {
    // Anonymous inline images avoid collisions with other invocations/programs.
    // C=1 leaves all cursor movement to the row writer; q=2 suppresses replies.
    format!(
        "\x1b_Ga=T,t=d,f=32,s={width},v={height},c={cols},r={rows},C=1,q=2{},m={};",
        if zlib { ",o=z" } else { "" },
        u8::from(more)
    )
}

// Complete command length for a payload of `bytes` bytes.
fn command_len(
    width: u32,
    height: u32,
    cols: usize,
    rows: usize,
    zlib: bool,
    bytes: usize,
) -> usize {
    let chunks = bytes.div_ceil(CHUNK).max(1);
    header(width, height, cols, rows, zlib, chunks > 1).len()
        + bytes.div_ceil(3) * 4
        + 2
        + (chunks - 1) * CONTINUATION.len()
}

/// Uncompressed command length: the upper bound budgets reserve for one image.
pub fn byte_len(width: u32, height: u32, cols: usize, rows: usize) -> usize {
    command_len(
        width,
        height,
        cols,
        rows,
        false,
        width as usize * height as usize * 4,
    )
}

/// RGBA pixels ready for transmission: a zlib stream (Kitty `o=z`) when that
/// makes the command shorter, otherwise the raw pixels. Artwork shrinks ~30x;
/// photos ~15%. Never longer than [`byte_len`], so budgets remain exact bounds.
pub struct Payload {
    width: u32,
    height: u32,
    zlib: bool,
    data: Vec<u8>,
}

impl Payload {
    pub fn new(image: RgbaImage) -> Self {
        let (width, height) = image.dimensions();
        // Level 1: nearly all of the saving at a fraction of the CPU.
        let compressed = miniz_oxide::deflate::compress_to_vec_zlib(image.as_raw(), 1);
        let zlib = command_len(width, height, 1, 1, true, compressed.len())
            < command_len(width, height, 1, 1, false, image.as_raw().len());
        Self {
            width,
            height,
            zlib,
            data: if zlib { compressed } else { image.into_raw() },
        }
    }

    pub fn write(&self, out: &mut impl Write, cols: usize, rows: usize) -> io::Result<()> {
        // Constant scratch space instead of a base64 copy of every payload.
        let mut encoded = [0; 4096];
        let mut chunks = self.data.chunks(CHUNK).peekable();
        let mut first = true;
        while let Some(chunk) = chunks.next() {
            let more = chunks.peek().is_some();
            if first {
                let header = header(self.width, self.height, cols, rows, self.zlib, more);
                out.write_all(header.as_bytes())?;
                first = false;
            } else {
                write!(out, "\x1b_Gm={},q=2;", u8::from(more))?;
            }
            let len = STANDARD
                .encode_slice(chunk, &mut encoded)
                .expect("base64 chunk fits");
            out.write_all(&encoded[..len])?;
            out.write_all(b"\x1b\\")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(bytes: &[u8]) -> (String, Vec<u8>) {
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        let chunks: Vec<_> = text.split("\x1b\\").filter(|s| !s.is_empty()).collect();
        let mut payload = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            let (control, data) = chunk.split_once(';').unwrap();
            assert!(control.starts_with("\x1b_G"));
            assert!(control.contains("q=2"));
            assert!(control.contains(if i + 1 == chunks.len() { "m=0" } else { "m=1" }));
            assert!(data.len() <= 4096);
            assert_eq!(data.len() % 4, 0);
            payload.extend(STANDARD.decode(data).unwrap());
        }
        let first = chunks[0].split_once(';').unwrap().0.to_owned();
        (first, payload)
    }

    #[test]
    fn payloads_roundtrip_within_the_uncompressed_budget() {
        let mut state = 1_u32;
        let mut noise = move || {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 24) as u8
        };
        for (width, height) in [(1, 1), (32, 24), (160, 80), (320, 240)] {
            let flat = RgbaImage::from_fn(width, height, |x, _| {
                image::Rgba([if x < width / 2 { 200 } else { 30 }, 90, 17, 255])
            });
            let random = RgbaImage::from_fn(width, height, |_, _| {
                image::Rgba([noise(), noise(), noise(), noise()])
            });
            for (image, compressible) in [(flat, width > 1), (random, false)] {
                let payload = Payload::new(image.clone());
                let mut bytes = Vec::new();
                payload.write(&mut bytes, 20, 5).unwrap();
                assert!(bytes.len() <= byte_len(width, height, 20, 5));
                let (control, data) = decode(&bytes);
                assert!(!control.contains("i=") && !control.contains("a=d"));
                let pixels = if control.contains(",o=z,") {
                    assert!(compressible, "{width}x{height}");
                    miniz_oxide::inflate::decompress_to_vec_zlib(&data).unwrap()
                } else {
                    // Incompressible pixels keep the exact uncompressed length.
                    assert!(!compressible);
                    assert_eq!(bytes.len(), byte_len(width, height, 20, 5));
                    data
                };
                assert_eq!(pixels, *image.as_raw());
            }
        }
    }
}
