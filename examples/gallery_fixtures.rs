//! Additional fixtures for large galleries, EXIF efficiency and SVG/ICO checks.
//! Kept separate from the original mixed fixtures and all user images.
use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
use std::{fs, io::Cursor, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new("target/visual-checks/gallery-files");
    if root.exists() {
        return Err("gallery-files already exists; keeping existing fixtures".into());
    }
    fs::create_dir_all(root)?;
    let img = DynamicImage::ImageRgb8(RgbImage::from_fn(3200, 2000, |x, y| {
        Rgb([
            (x * 255 / 3199) as u8,
            (y * 255 / 1999) as u8,
            if x / 200 % 2 == 0 { 220 } else { 40 },
        ])
    }));
    let mut bytes = Cursor::new(Vec::new());
    img.write_to(&mut bytes, ImageFormat::Jpeg)?;
    let exif = b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
    let mut jpeg = vec![0xff, 0xd8, 0xff, 0xe1];
    jpeg.extend(((exif.len() + 2) as u16).to_be_bytes());
    jpeg.extend(exif);
    jpeg.extend(&bytes.get_ref()[2..]);
    fs::write(root.join("rotated-photo.jpg"), jpeg)?;
    img.thumbnail(320, 200).save(root.join("landscape.png"))?;
    DynamicImage::ImageRgba8(img.thumbnail(32, 32).into_rgba8())
        .save_with_format(root.join("small-icon.ico"), ImageFormat::Ico)?;
    fs::write(
        root.join("vector.SVG"),
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 120">
<defs><linearGradient id="g"><stop stop-color="#f97316"/><stop offset="1" stop-color="#ec4899"/></linearGradient></defs>
<rect x="2" y="2" width="196" height="116" rx="20" fill="url(#g)"/>
<path d="M30 90L75 25L120 90Z" fill="white" fill-opacity=".8"/>
<circle cx="145" cy="50" r="22" fill="#312e81"/></svg>"##,
    )?;
    fs::write(
        root.join("unsupported-text.svg"),
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="40"><text y="30">Needs fonts</text></svg>"#,
    )?;
    fs::write(
        root.join("notes.txt"),
        "Check complete names, SVG/ICO content, EXIF portrait, and unsupported-SVG artwork.\n",
    )?;
    println!("Created {}", root.display());
    Ok(())
}
