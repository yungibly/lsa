//! macOS ImageIO thumbnails: HEIF/AVIF, TIFF, camera RAW, PSD and JPEG XL,
//! which the Rust decoders cannot read, plus reduced-resolution JPEG decoding.
//!
//! The frameworks load with dlopen on the first preview, so ordinary listings
//! pay no startup cost (linking them measured +0.7 ms per launch). ImageIO
//! reads through callbacks on the descriptor lsa already opened and
//! validated; it never reopens the path. Prototypes, struct layouts and
//! constants were checked against the macOS 14 SDK headers.
use image::RgbaImage;
use std::{
    ffi::{CStr, c_char, c_void},
    fs::File,
    io,
    os::fd::AsRawFd,
    ptr,
    sync::OnceLock,
};

type Ref = *const c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

// CGDataProviderDirectCallbacks: 40 bytes, getBytesAtPosition at offset 24.
#[repr(C)]
struct DirectCallbacks {
    version: u32,
    get_byte_pointer: Option<unsafe extern "C" fn(*mut c_void) -> *const c_void>,
    release_byte_pointer: Option<unsafe extern "C" fn(*mut c_void, *const c_void)>,
    get_bytes_at_position:
        Option<unsafe extern "C" fn(*mut c_void, *mut c_void, i64, usize) -> usize>,
    release_info: Option<unsafe extern "C" fn(*mut c_void)>,
}

const NUMBER_SINT64: isize = 4; // kCFNumberSInt64Type
const UTF8: u32 = 0x0800_0100; // kCFStringEncodingUTF8
const PREMULTIPLIED_RGBA: u32 = 1 | (4 << 12); // AlphaPremultipliedLast | ByteOrder32Big
const INTERPOLATION_HIGH: i32 = 3;

#[allow(clippy::type_complexity)]
struct Api {
    release: unsafe extern "C" fn(Ref),
    dictionary_create: unsafe extern "C" fn(Ref, *const Ref, *const Ref, isize, Ref, Ref) -> Ref,
    dictionary_get_value: unsafe extern "C" fn(Ref, Ref) -> Ref,
    number_create: unsafe extern "C" fn(Ref, isize, *const c_void) -> Ref,
    number_get_value: unsafe extern "C" fn(Ref, isize, *mut c_void) -> u8,
    get_type_id: unsafe extern "C" fn(Ref) -> usize,
    number_type_id: unsafe extern "C" fn() -> usize,
    string_get_c_string: unsafe extern "C" fn(Ref, *mut c_char, isize, u32) -> u8,
    provider_create_direct: unsafe extern "C" fn(*mut c_void, i64, *const DirectCallbacks) -> Ref,
    source_create_with_data_provider: unsafe extern "C" fn(Ref, Ref) -> Ref,
    source_get_type: unsafe extern "C" fn(Ref) -> Ref,
    source_get_count: unsafe extern "C" fn(Ref) -> usize,
    source_get_primary_image_index: unsafe extern "C" fn(Ref) -> usize,
    source_copy_properties_at_index: unsafe extern "C" fn(Ref, usize, Ref) -> Ref,
    source_create_thumbnail_at_index: unsafe extern "C" fn(Ref, usize, Ref) -> Ref,
    image_get_width: unsafe extern "C" fn(Ref) -> usize,
    image_get_height: unsafe extern "C" fn(Ref) -> usize,
    color_space_create_with_name: unsafe extern "C" fn(Ref) -> Ref,
    bitmap_context_create:
        unsafe extern "C" fn(*mut c_void, usize, usize, usize, usize, Ref, u32) -> Ref,
    context_set_interpolation_quality: unsafe extern "C" fn(Ref, i32),
    context_draw_image: unsafe extern "C" fn(Ref, Rect, Ref),
    boolean_true: Ref,
    boolean_false: Ref,
    key_callbacks: Ref,
    value_callbacks: Ref,
    srgb: Ref,
    from_image_always: Ref,
    from_image_if_absent: Ref,
    max_pixel_size: Ref,
    with_transform: Ref,
    cache_immediately: Ref,
    should_cache: Ref,
    pixel_width: Ref,
    pixel_height: Ref,
    orientation: Ref,
}

// The pointers are immutable framework functions and constant CF objects.
unsafe impl Send for Api {}
unsafe impl Sync for Api {}

fn api() -> Option<&'static Api> {
    static API: OnceLock<Option<Api>> = OnceLock::new();
    API.get_or_init(|| unsafe { load() }).as_ref()
}

unsafe fn open(path: &CStr) -> Option<*mut c_void> {
    let library = unsafe { libc::dlopen(path.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL) };
    (!library.is_null()).then_some(library)
}

// A function symbol, transmuted to its checked prototype.
unsafe fn function<T: Copy>(library: *mut c_void, name: &CStr) -> Option<T> {
    let symbol = unsafe { libc::dlsym(library, name.as_ptr()) };
    (!symbol.is_null()).then(|| unsafe { std::mem::transmute_copy(&symbol) })
}

// The address of an exported data symbol.
unsafe fn address(library: *mut c_void, name: &CStr) -> Option<Ref> {
    let symbol = unsafe { libc::dlsym(library, name.as_ptr()) };
    (!symbol.is_null()).then_some(symbol.cast_const())
}

// The value of an exported constant object such as a CFStringRef key.
unsafe fn constant(library: *mut c_void, name: &CStr) -> Option<Ref> {
    let value = unsafe { address(library, name)?.cast::<Ref>().read() };
    (!value.is_null()).then_some(value)
}

unsafe fn load() -> Option<Api> {
    unsafe {
        let cf = open(c"/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation")?;
        let cg = open(c"/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics")?;
        let io = open(c"/System/Library/Frameworks/ImageIO.framework/ImageIO")?;
        Some(Api {
            release: function(cf, c"CFRelease")?,
            dictionary_create: function(cf, c"CFDictionaryCreate")?,
            dictionary_get_value: function(cf, c"CFDictionaryGetValue")?,
            number_create: function(cf, c"CFNumberCreate")?,
            number_get_value: function(cf, c"CFNumberGetValue")?,
            get_type_id: function(cf, c"CFGetTypeID")?,
            number_type_id: function(cf, c"CFNumberGetTypeID")?,
            string_get_c_string: function(cf, c"CFStringGetCString")?,
            provider_create_direct: function(cg, c"CGDataProviderCreateDirect")?,
            source_create_with_data_provider: function(io, c"CGImageSourceCreateWithDataProvider")?,
            source_get_type: function(io, c"CGImageSourceGetType")?,
            source_get_count: function(io, c"CGImageSourceGetCount")?,
            source_get_primary_image_index: function(io, c"CGImageSourceGetPrimaryImageIndex")?,
            source_copy_properties_at_index: function(io, c"CGImageSourceCopyPropertiesAtIndex")?,
            source_create_thumbnail_at_index: function(io, c"CGImageSourceCreateThumbnailAtIndex")?,
            image_get_width: function(cg, c"CGImageGetWidth")?,
            image_get_height: function(cg, c"CGImageGetHeight")?,
            color_space_create_with_name: function(cg, c"CGColorSpaceCreateWithName")?,
            bitmap_context_create: function(cg, c"CGBitmapContextCreate")?,
            context_set_interpolation_quality: function(cg, c"CGContextSetInterpolationQuality")?,
            context_draw_image: function(cg, c"CGContextDrawImage")?,
            boolean_true: constant(cf, c"kCFBooleanTrue")?,
            boolean_false: constant(cf, c"kCFBooleanFalse")?,
            key_callbacks: address(cf, c"kCFTypeDictionaryKeyCallBacks")?,
            value_callbacks: address(cf, c"kCFTypeDictionaryValueCallBacks")?,
            srgb: constant(cg, c"kCGColorSpaceSRGB")?,
            from_image_always: constant(io, c"kCGImageSourceCreateThumbnailFromImageAlways")?,
            from_image_if_absent: constant(io, c"kCGImageSourceCreateThumbnailFromImageIfAbsent")?,
            max_pixel_size: constant(io, c"kCGImageSourceThumbnailMaxPixelSize")?,
            with_transform: constant(io, c"kCGImageSourceCreateThumbnailWithTransform")?,
            cache_immediately: constant(io, c"kCGImageSourceShouldCacheImmediately")?,
            should_cache: constant(io, c"kCGImageSourceShouldCache")?,
            pixel_width: constant(io, c"kCGImagePropertyPixelWidth")?,
            pixel_height: constant(io, c"kCGImagePropertyPixelHeight")?,
            orientation: constant(io, c"kCGImagePropertyOrientation")?,
        })
    }
}

/// An owned (+1) Core Foundation object, released on drop.
struct Owned<'a>(Ref, &'a Api);
impl<'a> Owned<'a> {
    fn new(api: &'a Api, object: Ref, what: &'static str) -> Result<Self, &'static str> {
        if object.is_null() {
            Err(what)
        } else {
            Ok(Self(object, api))
        }
    }
}
impl Drop for Owned<'_> {
    fn drop(&mut self) {
        unsafe { (self.1.release)(self.0) }
    }
}

// The provider owns this descriptor and closes it through release_info.
struct Source {
    file: File,
    len: u64,
}

unsafe extern "C" fn read_at(
    info: *mut c_void,
    buffer: *mut c_void,
    position: i64,
    count: usize,
) -> usize {
    // Called from ImageIO: never panic, never read past the validated length.
    let source = unsafe { &*info.cast::<Source>() };
    let Ok(position) = u64::try_from(position) else {
        return 0;
    };
    let Some(available) = source.len.checked_sub(position) else {
        return 0;
    };
    let count = count.min(usize::try_from(available).unwrap_or(usize::MAX));
    let mut done = 0;
    while done < count {
        let read = unsafe {
            libc::pread(
                source.file.as_raw_fd(),
                buffer.cast::<u8>().add(done).cast(),
                count - done,
                (position + done as u64) as libc::off_t,
            )
        };
        match read {
            1.. => done += read as usize,
            0 => break,
            _ if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted => {}
            _ => break,
        }
    }
    done
}

unsafe extern "C" fn release_source(info: *mut c_void) {
    drop(unsafe { Box::from_raw(info.cast::<Source>()) });
}

static CALLBACKS: DirectCallbacks = DirectCallbacks {
    version: 0,
    get_byte_pointer: None,
    release_byte_pointer: None,
    get_bytes_at_position: Some(read_at),
    release_info: Some(release_source),
};

/// Content types lsa accepts from ImageIO; anything else (a `.jpg` holding a
/// PNG, say) is refused here and left to the Rust decoders.
fn supported(kind: &str) -> bool {
    matches!(
        kind,
        "public.jpeg"
            | "public.heic"
            | "public.heif"
            | "public.avif"
            | "public.tiff"
            | "public.jpeg-xl"
            | "com.adobe.photoshop-image"
    ) || kind.ends_with("raw-image")
}

unsafe fn number(api: &Api, dictionary: Ref, key: Ref) -> Option<i64> {
    unsafe {
        let value = (api.dictionary_get_value)(dictionary, key);
        if value.is_null() || (api.get_type_id)(value) != (api.number_type_id)() {
            return None;
        }
        let mut result = 0_i64;
        ((api.number_get_value)(value, NUMBER_SINT64, (&raw mut result).cast()) != 0)
            .then_some(result)
    }
}

unsafe fn dictionary<'a>(api: &'a Api, pairs: &[(Ref, Ref)]) -> Result<Owned<'a>, &'static str> {
    let keys: Vec<Ref> = pairs.iter().map(|pair| pair.0).collect();
    let values: Vec<Ref> = pairs.iter().map(|pair| pair.1).collect();
    let dictionary = unsafe {
        (api.dictionary_create)(
            ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            pairs.len() as isize,
            api.key_callbacks,
            api.value_callbacks,
        )
    };
    Owned::new(api, dictionary, "no options dictionary")
}

/// Largest size with the same aspect ratio fitting `width`×`height`; small
/// sources scale up, matching the Rust decoders' thumbnails.
fn fit(source: (u64, u64), width: u32, height: u32) -> (u32, u32) {
    let (w, h) = (source.0.max(1) as f64, source.1.max(1) as f64);
    let scale = (f64::from(width) / w).min(f64::from(height) / h);
    (
        ((w * scale).round() as u32).clamp(1, width),
        ((h * scale).round() as u32).clamp(1, height),
    )
}

// ImageIO decodes these directly at thumbnail resolution: a 48 MP JPEG, HEIC
// or AVIF peaked at 13-25 MiB, where a full decode needs ~190 MB. TIFF, by
// contrast, decoded at full size (159 MiB for the same 48 MP image).
fn reduced(kind: &str) -> bool {
    matches!(
        kind,
        "public.jpeg" | "public.heic" | "public.heif" | "public.avif"
    )
}

/// A thumbnail fitting `width`×`height` (straight alpha, not yet on the
/// checker canvas) from a regular file already validated at `len` bytes.
/// `pixel_limit` bounds full-size decodes; formats ImageIO decodes at reduced
/// resolution may be 64 times larger. Beyond that, embedded previews only.
pub fn thumbnail(
    file: &File,
    len: u64,
    width: u32,
    height: u32,
    pixel_limit: u64,
) -> Result<RgbaImage, &'static str> {
    let api = api().ok_or("ImageIO unavailable")?;
    let file = file.try_clone().map_err(|_| "cannot share source")?;
    let info = Box::into_raw(Box::new(Source { file, len })).cast::<c_void>();
    unsafe {
        let provider = (api.provider_create_direct)(info, len as i64, &CALLBACKS);
        if provider.is_null() {
            // Not handed over; free it here rather than risk a double release.
            release_source(info);
            return Err("no data provider");
        }
        let provider = Owned(provider, api);
        let options = dictionary(api, &[(api.should_cache, api.boolean_false)])?;
        let source = Owned::new(
            api,
            (api.source_create_with_data_provider)(provider.0, options.0),
            "no image source",
        )?;
        drop(provider);
        let kind = (api.source_get_type)(source.0);
        let mut name = [0 as c_char; 128];
        if kind.is_null()
            || (api.string_get_c_string)(kind, name.as_mut_ptr(), name.len() as isize, UTF8) == 0
        {
            return Err("unknown image type");
        }
        let kind = CStr::from_ptr(name.as_ptr()).to_str().unwrap_or("");
        if !supported(kind) {
            return Err("image type not accepted from ImageIO");
        }
        if (api.source_get_count)(source.0) == 0 {
            return Err("no images");
        }
        let index = (api.source_get_primary_image_index)(source.0);
        let properties = Owned::new(
            api,
            (api.source_copy_properties_at_index)(source.0, index, ptr::null()),
            "no image properties",
        )?;
        let (Some(w), Some(h)) = (
            number(api, properties.0, api.pixel_width),
            number(api, properties.0, api.pixel_height),
        ) else {
            return Err("image has no dimensions");
        };
        let (w, h) = (u64::try_from(w).unwrap_or(0), u64::try_from(h).unwrap_or(0));
        if w == 0 || h == 0 {
            return Err("image has no dimensions");
        }
        // Orientations 5-8 transpose the displayed image.
        let rotated = matches!(number(api, properties.0, api.orientation), Some(5..=8));
        let shown = if rotated { (h, w) } else { (w, h) };
        let (fit_w, fit_h) = fit(shown, width, height);
        let jpeg = kind == "public.jpeg";
        let pixels = w.saturating_mul(h);
        let full =
            pixels <= pixel_limit || (reduced(kind) && pixels <= pixel_limit.saturating_mul(64));
        let size = i64::from(fit_w.max(fit_h));
        let size = Owned::new(
            api,
            (api.number_create)(ptr::null(), NUMBER_SINT64, (&raw const size).cast()),
            "no size number",
        )?;
        let create = |always: bool| -> Result<Owned<'_>, &'static str> {
            let flag = |on: bool| {
                if on {
                    api.boolean_true
                } else {
                    api.boolean_false
                }
            };
            let options = dictionary(
                api,
                &[
                    (api.max_pixel_size, size.0),
                    (api.with_transform, api.boolean_true),
                    (api.cache_immediately, api.boolean_true),
                    (api.from_image_always, flag(always)),
                    (api.from_image_if_absent, flag(always)),
                ],
            )?;
            Owned::new(
                api,
                (api.source_create_thumbnail_at_index)(source.0, index, options.0),
                "no thumbnail",
            )
        };
        // JPEG decodes directly (its EXIF thumbnails are tiny). Others prefer
        // an embedded preview and decode the image, within bounds, when that
        // preview is missing or too small. (HEIF and AVIF return nothing,
        // rather than decoding, when asked to create one only if absent.)
        let small = |image: &Owned| {
            let side = (api.image_get_width)(image.0).max((api.image_get_height)(image.0));
            side * 4 < fit_w.max(fit_h) as usize * 3
        };
        let mut image = if jpeg && full {
            None
        } else {
            create(false).ok()
        };
        if full && image.as_ref().is_none_or(small) {
            image = Some(create(true)?);
        }
        let image = image.ok_or("no embedded preview within limits")?;
        // Draw into an sRGB bitmap of exactly the fitted size: this converts
        // color profiles and pixel formats, and scales the embedded preview.
        let (image_w, image_h) = (
            (api.image_get_width)(image.0) as u64,
            (api.image_get_height)(image.0) as u64,
        );
        let (out_w, out_h) = fit((image_w, image_h), width, height);
        let mut pixels = vec![0_u8; out_w as usize * out_h as usize * 4];
        let space = Owned::new(
            api,
            (api.color_space_create_with_name)(api.srgb),
            "no color space",
        )?;
        let context = Owned::new(
            api,
            (api.bitmap_context_create)(
                pixels.as_mut_ptr().cast(),
                out_w as usize,
                out_h as usize,
                8,
                out_w as usize * 4,
                space.0,
                PREMULTIPLIED_RGBA,
            ),
            "no bitmap context",
        )?;
        (api.context_set_interpolation_quality)(context.0, INTERPOLATION_HIGH);
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            width: f64::from(out_w),
            height: f64::from(out_h),
        };
        (api.context_draw_image)(context.0, rect, image.0);
        drop(context);
        // Bitmap contexts store premultiplied alpha; the canvas expects straight.
        for pixel in pixels.as_chunks_mut::<4>().0 {
            let alpha = u32::from(pixel[3]);
            if alpha != 0 && alpha != 255 {
                for channel in &mut pixel[..3] {
                    *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
                }
            }
        }
        RgbaImage::from_raw(out_w, out_h, pixels).ok_or("invalid bitmap")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageFormat, metadata::Orientation};
    use std::{
        io::Cursor,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static NEXT: AtomicUsize = AtomicUsize::new(0);
    fn source(bytes: &[u8]) -> (File, u64, PathBuf) {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/imageio-tests");
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join(format!(
            "{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, bytes).unwrap();
        (File::open(&path).unwrap(), bytes.len() as u64, path)
    }
    fn thumb(bytes: &[u8], width: u32, height: u32, limit: u64) -> Result<RgbaImage, &'static str> {
        let (file, len, path) = source(bytes);
        let result = thumbnail(&file, len, width, height, limit);
        std::fs::remove_file(path).unwrap();
        result
    }
    // Quadrants: red top-left, green top-right, blue bottom-left, white.
    fn quadrants() -> DynamicImage {
        DynamicImage::ImageRgb8(image::RgbImage::from_fn(120, 80, |x, y| {
            image::Rgb(match (x < 60, y < 40) {
                (true, true) => [230, 20, 20],
                (false, true) => [20, 230, 20],
                (true, false) => [20, 20, 230],
                (false, false) => [240, 240, 240],
            })
        }))
    }
    fn jpeg(image: &DynamicImage, orientation: u8) -> Vec<u8> {
        let mut data = Cursor::new(Vec::new());
        image.write_to(&mut data, ImageFormat::Jpeg).unwrap();
        let mut exif =
            b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x01\0\0\0\0\0\0\0".to_vec();
        exif[24] = orientation;
        let mut bytes = vec![0xff, 0xd8, 0xff, 0xe1];
        bytes.extend(((exif.len() + 2) as u16).to_be_bytes());
        bytes.extend(exif);
        bytes.extend(&data.get_ref()[2..]);
        bytes
    }
    fn dominant(pixel: &image::Rgba<u8>) -> usize {
        let [r, g, b, _] = pixel.0;
        if r > 200 && g > 200 && b > 200 {
            3
        } else {
            [r, g, b]
                .iter()
                .enumerate()
                .max_by_key(|(_, v)| **v)
                .unwrap()
                .0
        }
    }

    #[test]
    fn every_exif_orientation_is_upright_and_matches_the_reference() {
        for value in 1..=8 {
            let bytes = jpeg(&quadrants(), value);
            let actual = thumb(&bytes, 48, 48, 16_000_000).unwrap();
            let mut reference = image::load_from_memory(&bytes).unwrap();
            reference.apply_orientation(Orientation::from_exif(value).unwrap());
            let reference = reference.thumbnail(48, 48).into_rgba8();
            assert_eq!(
                actual.dimensions(),
                reference.dimensions(),
                "orientation {value}"
            );
            let (w, h) = actual.dimensions();
            for (x, y) in [
                (w / 4, h / 4),
                (3 * w / 4, h / 4),
                (w / 4, 3 * h / 4),
                (3 * w / 4, 3 * h / 4),
            ] {
                assert_eq!(
                    dominant(actual.get_pixel(x, y)),
                    dominant(reference.get_pixel(x, y)),
                    "orientation {value} at {x},{y}"
                );
            }
        }
    }

    #[test]
    fn content_types_and_pixel_limits_are_enforced() {
        // A PNG is left to the Rust decoders, even when named like a JPEG.
        let mut png = Cursor::new(Vec::new());
        quadrants().write_to(&mut png, ImageFormat::Png).unwrap();
        assert_eq!(
            thumb(png.get_ref(), 40, 40, 16_000_000).unwrap_err(),
            "image type not accepted from ImageIO"
        );
        // 120x80 = 9,600 pixels: JPEG decodes at reduced resolution up to 64x
        // the limit; beyond that only an embedded preview would be accepted.
        let bytes = jpeg(&quadrants(), 1);
        assert!(thumb(&bytes, 40, 40, 1_000).is_ok());
        assert_eq!(
            thumb(&bytes, 40, 40, 100).unwrap_err(),
            "no embedded preview within limits"
        );
        // Small sources scale up to fill the box, like the Rust decoders.
        assert_eq!(
            thumb(&bytes, 240, 160, 16_000_000).unwrap().dimensions(),
            (240, 160)
        );
    }

    #[test]
    fn empty_truncated_and_random_sources_fail_cleanly() {
        assert!(thumb(b"", 40, 40, 16_000_000).is_err());
        assert!(thumb(b"\xff\xd8\xff", 40, 40, 16_000_000).is_err());
        let mut state = 7_u32;
        let random: Vec<u8> = (0..65_536)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state >> 24) as u8
            })
            .collect();
        assert!(thumb(&random, 40, 40, 16_000_000).is_err());
        // A truncated JPEG may decode partially or fail, but never panics.
        let bytes = jpeg(&quadrants(), 6);
        let _ = thumb(&bytes[..bytes.len() / 2], 40, 40, 16_000_000);
    }
}
