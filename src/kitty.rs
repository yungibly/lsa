use base64::{Engine, engine::general_purpose::STANDARD};
use image::RgbaImage;
use std::io::{self, Write};

fn header(width: u32, height: u32, cols: usize, rows: usize, more: bool) -> String {
    // Anonymous inline images avoid collisions with other invocations/programs.
    // C=1 leaves all cursor movement to the row writer; q=2 suppresses replies.
    format!(
        "\x1b_Ga=T,t=d,f=32,s={width},v={height},c={cols},r={rows},C=1,q=2,m={};",
        u8::from(more)
    )
}

pub fn byte_len(width: u32, height: u32, cols: usize, rows: usize) -> usize {
    let payload = (width as usize * height as usize * 4).div_ceil(3) * 4;
    let chunks = payload.div_ceil(4096);
    header(width, height, cols, rows, chunks > 1).len()
        + payload
        + 2
        + (chunks - 1) * "\x1b_Gm=1,q=2;\x1b\\".len()
}

pub fn write(out: &mut impl Write, image: &RgbaImage, cols: usize, rows: usize) -> io::Result<()> {
    let encoded = STANDARD.encode(image.as_raw());
    let mut chunks = encoded.as_bytes().chunks(4096).peekable();
    let mut first = true;
    while let Some(chunk) = chunks.next() {
        let more = chunks.peek().is_some();
        if first {
            out.write_all(header(image.width(), image.height(), cols, rows, more).as_bytes())?;
            first = false;
        } else {
            write!(out, "\x1b_Gm={},q=2;", u8::from(more))?;
        }
        out.write_all(chunk)?;
        out.write_all(b"\x1b\\")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chunks_roundtrip_and_match_budget_exactly() {
        for (width, height) in [(1, 1), (32, 24), (160, 80), (320, 240)] {
            let img = RgbaImage::from_fn(width, height, |x, y| {
                image::Rgba([x as u8, y as u8, 17, 255])
            });
            let mut bytes = Vec::new();
            write(&mut bytes, &img, 20, 5).unwrap();
            assert_eq!(bytes.len(), byte_len(width, height, 20, 5));
            let text = String::from_utf8(bytes).unwrap();
            let chunks: Vec<_> = text.split("\x1b\\").filter(|s| !s.is_empty()).collect();
            let mut pixels = Vec::new();
            for (i, chunk) in chunks.iter().enumerate() {
                let (control, data) = chunk.split_once(';').unwrap();
                assert!(control.starts_with("\x1b_G"));
                assert!(control.contains("q=2"));
                assert!(control.contains(if i + 1 == chunks.len() { "m=0" } else { "m=1" }));
                assert!(data.len() <= 4096);
                assert_eq!(data.len() % 4, 0);
                pixels.extend(STANDARD.decode(data).unwrap());
            }
            assert_eq!(pixels, *img.as_raw());
            assert!(!text.contains("i="));
            assert!(!text.contains("a=d"));
        }
    }
}
