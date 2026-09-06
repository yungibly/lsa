//! Offline artwork QA. Run from the repository; no terminal or font dependency.
#[path = "../src/artwork.rs"]
mod artwork;
use artwork::Icon;
use image::{Rgba, RgbaImage, imageops};
fn main() {
    let icons = [
        Icon::Folder,
        Icon::File,
        Icon::Image,
        Icon::Video,
        Icon::Audio,
        Icon::Archive,
        Icon::Code,
        Icon::Config,
        Icon::Link,
        Icon::Special,
        Icon::Error,
    ];
    let mut sheet = RgbaImage::new(icons.len() as u32 * 120, 224);
    for (row, bg) in [[38, 41, 52, 255], [248, 247, 243, 255]]
        .into_iter()
        .enumerate()
    {
        let y = row as i64 * 112;
        imageops::overlay(
            &mut sheet,
            &RgbaImage::from_pixel(1320, 112, Rgba(bg)),
            0,
            y,
        );
        for (column, icon) in icons.into_iter().enumerate() {
            imageops::overlay(
                &mut sheet,
                &artwork::render(icon, 112, 51, true),
                column as i64 * 120 + 4,
                y + 8,
            );
            imageops::overlay(
                &mut sheet,
                &artwork::render(icon, 24, 17, true),
                column as i64 * 120 + 48,
                y + 78,
            );
        }
    }
    std::fs::create_dir_all("target/visual-checks").unwrap();
    sheet.save("target/visual-checks/artwork.png").unwrap();
}
