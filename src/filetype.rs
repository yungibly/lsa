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

pub struct Classification {
    pub category: Category,
    pub preview: bool,
}

/// One filename-only classification for styling and decoder eligibility. Unknown
/// extensions remain ordinary files; no content sniffing on the text path.
pub fn classify(path: &Path) -> Classification {
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let is = |extensions: &[&str]| extensions.iter().any(|s| extension.eq_ignore_ascii_case(s));
    let preview = is(&["jpg", "jpeg", "png", "gif", "webp", "bmp"]);
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let category = if preview
        || is(&[
            "svg", "avif", "heic", "heif", "tif", "tiff", "ico", "jxl", "raw", "dng", "psd",
        ]) {
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
        for extension in ["gif", "GIF", "JpEg", "webp", "png", "bmp"] {
            let kind = classify(Path::new(&format!("photo.{extension}")));
            assert_eq!(kind.category, Category::Image);
            assert!(kind.preview);
        }
        for extension in ["svg", "HEIC", "avif", "jxl", "tiff", "psd"] {
            let kind = classify(Path::new(&format!("photo.{extension}")));
            assert_eq!(kind.category, Category::Image);
            assert!(!kind.preview);
        }
        assert_eq!(
            classify(Path::new("unknown.new-format")).category,
            Category::File
        );
    }
}
