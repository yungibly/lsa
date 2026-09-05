//! Reproducible synthetic fixtures; never modifies the user's original images.
use image::{DynamicImage, ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};
use std::{fs, os::unix::fs::symlink, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new("img-test/generated");
    if root.exists() {
        return Err("img-test/generated already exists; keeping existing fixtures".into());
    }
    fs::create_dir_all(root.join("folder"))?;
    fs::write(
        root.join("notes.txt"),
        "Ordinary files belong beside image previews.\n",
    )?;
    fs::write(
        root.join("broken.png"),
        "This is deliberately not an image.",
    )?;
    fs::write(root.join(".hidden"), "hidden entry")?;
    let color = RgbImage::from_fn(320, 160, |x, y| {
        if x < 16 || y < 16 || x >= 304 || y >= 144 {
            Rgb([255, 255, 255])
        } else if x < 160 {
            Rgb([225, 40, 55])
        } else {
            Rgb([30, 120, 240])
        }
    });
    for (name, format) in [
        ("landscape.png", ImageFormat::Png),
        ("landscape.jpg", ImageFormat::Jpeg),
        ("still.gif", ImageFormat::Gif),
        ("landscape.webp", ImageFormat::WebP),
        ("landscape.bmp", ImageFormat::Bmp),
    ] {
        DynamicImage::ImageRgb8(color.clone()).save_with_format(root.join(name), format)?;
    }
    DynamicImage::ImageRgb8(color.clone())
        .rotate90()
        .save(root.join("portrait.png"))?;
    RgbaImage::from_fn(160, 160, |x, y| {
        let dx = i64::from(x) - 80;
        let dy = i64::from(y) - 80;
        Rgba([
            40,
            220,
            120,
            if dx * dx + dy * dy < 60 * 60 { 180 } else { 0 },
        ])
    })
    .save(root.join("transparent.png"))?;
    DynamicImage::ImageRgb8(color)
        .save(root.join("long filename with spaces and 桃 and emoji 👩‍💻.png"))?;
    symlink("landscape.png", root.join("linked.png"))?;
    symlink("missing.png", root.join("dangling.png"))?;
    symlink("folder", root.join("linked-folder"))?;
    fs::create_dir(root.join("many"))?;
    for i in 0..40 {
        symlink(
            "../landscape.png",
            root.join(format!("many/image-{i:02}.png")),
        )?;
    }
    println!("Created {}", root.display());
    Ok(())
}
