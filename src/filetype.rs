use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Image,
    Video,
    Audio,
    Archive,
    Code,
    Config,
    Document,
    File,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Classification {
    pub category: Category,
    pub preview: bool,
}

// Decoded by the bundled Rust decoders on every platform.
const RUST_PREVIEW: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp", "svg", "ico"];
/// Decoded by macOS ImageIO: HEIF/AVIF, TIFF, JPEG XL, Photoshop and camera
/// RAW (through its embedded previews).
const SYSTEM_PREVIEW: &[&str] = &[
    "heic", "heif", "avif", "tif", "tiff", "jxl", "psd", "dng", "cr2", "cr3", "nef", "nrw", "arw",
    "raf", "orf", "rw2", "pef", "srw",
];

fn extension(path: &Path) -> &str {
    path.extension().and_then(|s| s.to_str()).unwrap_or("")
}

fn any(extension: &str, extensions: &[&str]) -> bool {
    extensions.iter().any(|s| extension.eq_ignore_ascii_case(s))
}

/// Sources macOS decodes with ImageIO, including JPEG for its reduced-
/// resolution decoding. Content is still checked before decoding.
pub fn system_decoder(path: &Path) -> bool {
    cfg!(target_os = "macos") && {
        let extension = extension(path);
        any(extension, SYSTEM_PREVIEW) || any(extension, &["jpg", "jpeg"])
    }
}

/// One filename-only classification for styling and decoder eligibility. Unknown
/// extensions remain ordinary files; no content sniffing on the text path.
pub fn classify(path: &Path) -> Classification {
    let extension = extension(path);
    let is = |extensions: &[&str]| any(extension, extensions);
    let preview = is(RUST_PREVIEW) || (cfg!(target_os = "macos") && is(SYSTEM_PREVIEW));
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let category = if preview || is(SYSTEM_PREVIEW) || is(&["raw"]) {
        Category::Image
    } else if is(&["mp4", "mov", "mkv", "webm", "avi", "m4v", "mpeg", "mpg"]) {
        Category::Video
    } else if is(&["mp3", "wav", "flac", "ogg", "m4a", "aac", "aiff", "opus"]) {
        Category::Audio
    } else if is(&[
        "zip", "gz", "xz", "bz2", "zst", "tar", "7z", "rar", "dmg", "iso", "tgz",
    ]) {
        Category::Archive
    } else if is(&[
        "rs", "py", "js", "ts", "tsx", "jsx", "go", "c", "h", "cpp", "hpp", "swift", "rb", "java",
        "sh", "bash", "zsh", "html", "css", "sql", "lua", "kt", "svelte", "vue",
    ]) {
        Category::Code
    } else if is(&["json", "toml", "yaml", "yml", "xml", "ini", "conf", "lock"])
        || [".gitignore", "Makefile", "Dockerfile"].contains(&name)
    {
        Category::Config
    } else if is(&[
        "md", "txt", "pdf", "doc", "docx", "rtf", "csv", "xlsx", "log", "odt", "epub",
    ]) || ["LICENSE", "README"].contains(&name)
    {
        Category::Document
    } else {
        Category::File
    };
    Classification { category, preview }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recognized_images_have_consistent_styles_even_without_a_decoder() {
        for extension in ["gif", "GIF", "JpEg", "webp", "png", "bmp", "SVG", "ico"] {
            let kind = classify(Path::new(&format!("photo.{extension}")));
            assert_eq!(kind.category, Category::Image);
            assert!(kind.preview);
        }
        // System formats preview through ImageIO on macOS only.
        for extension in ["HEIC", "avif", "jxl", "tiff", "psd", "CR3", "dng"] {
            let kind = classify(Path::new(&format!("photo.{extension}")));
            assert_eq!(kind.category, Category::Image);
            assert_eq!(kind.preview, cfg!(target_os = "macos"), "{extension}");
            assert_eq!(
                system_decoder(Path::new(&format!("a.{extension}"))),
                cfg!(target_os = "macos")
            );
        }
        // A generic .raw stays an image without a decoder everywhere.
        let raw = classify(Path::new("scan.raw"));
        assert!(raw.category == Category::Image && !raw.preview);
        assert_eq!(
            system_decoder(Path::new("a.JPG")),
            cfg!(target_os = "macos")
        );
        assert!(!system_decoder(Path::new("a.png")) && !system_decoder(Path::new("a.svg")));
        assert_eq!(
            classify(Path::new("unknown.new-format")).category,
            Category::File
        );
    }
}
