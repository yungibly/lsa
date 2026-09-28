//! Deterministic, poorly compressible JPEG for decoder memory checks:
//! `jpeg_fixture PATH WIDTH HEIGHT`. Refuses to overwrite an existing file.
use image::{Rgb, RgbImage, codecs::jpeg::JpegEncoder};
use std::{fs::OpenOptions, io::BufWriter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [path, width, height] = args.as_slice() else {
        return Err("usage: jpeg_fixture PATH WIDTH HEIGHT".into());
    };
    let (width, height): (u32, u32) = (width.parse()?, height.parse()?);
    let mut state = 0x2545_f491_u32;
    let image = RgbImage::from_fn(width, height, |_, _| {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        let [r, g, b, _] = state.to_le_bytes();
        Rgb([r, g, b])
    });
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    JpegEncoder::new_with_quality(BufWriter::new(file), 90).encode_image(&image)?;
    Ok(())
}
