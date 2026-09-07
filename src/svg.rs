//! Bounded, self-contained vector artwork. No font discovery, resource loading,
//! embedded raster decoding, or rendering at the document's native dimensions.
use image::{Rgba, RgbaImage};
use resvg::{tiny_skia, usvg};
use std::io;

pub const INPUT_LIMIT: usize = 256 * 1024;
const NODE_LIMIT: u32 = 4096;
const DEPTH_LIMIT: usize = 32;

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub fn decode(
    bytes: &[u8],
    width: u32,
    height: u32,
) -> Result<RgbaImage, Box<dyn std::error::Error>> {
    if bytes.len() > INPUT_LIMIT || !(1..=320).contains(&width) || !(1..=240).contains(&height) {
        return Err(invalid("SVG exceeds source or thumbnail limits").into());
    }
    let text = std::str::from_utf8(bytes)?;
    let doc = roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: NODE_LIMIT,
            ..roxmltree::ParsingOptions::default()
        },
    )?;
    for node in doc.descendants().filter(|n| n.is_element()) {
        if node.ancestors().take(DEPTH_LIMIT + 1).count() > DEPTH_LIMIT {
            return Err(invalid("SVG exceeds nesting limit").into());
        }
        // Referenced/repeated subtrees, nested surfaces, filters and fonts have
        // different work/allocation costs. Reject the whole preview rather
        // than silently render an incomplete document or expand their work.
        if matches!(
            node.tag_name().name(),
            "use" | "pattern" | "marker" | "mask" | "filter" | "image" | "text" | "foreignObject"
        ) {
            return Err(invalid("SVG needs unsupported resources or effects").into());
        }
    }
    let options = usvg::Options {
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(|_, _, _| None),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_xmltree(&doc, &options)?;
    let size = tree.size();
    let scale = (width as f32 / size.width()).min(height as f32 / size.height());
    let w = (size.width() * scale).round().clamp(1.0, width as f32) as u32;
    let h = (size.height() * scale).round().clamp(1.0, height as f32) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(w, h).ok_or_else(|| invalid("invalid SVG canvas"))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    // tiny-skia returns premultiplied alpha; the shared checkerboard compositor
    // expects straight alpha. Convert only this thumbnail-sized buffer.
    let mut thumb = RgbaImage::new(w, h);
    for (pixel, color) in thumb.pixels_mut().zip(pixmap.pixels()) {
        let color = color.demultiply();
        *pixel = Rgba([color.red(), color.green(), color.blue(), color.alpha()]);
    }
    Ok(crate::preview::canvas(&thumb, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn svg(body: &str) -> String {
        format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 20">{body}</svg>"#)
    }
    #[test]
    fn renders_vector_shapes_with_aspect_and_transparency() {
        let data = svg(r#"<rect width="40" height="20" fill="red" fill-opacity="0.5"/>"#);
        let image = decode(data.as_bytes(), 40, 40).unwrap();
        assert_eq!(image.get_pixel(0, 0)[3], 0);
        let center = image.get_pixel(2, 12);
        assert!((214..=217).contains(&center[0]), "{center:?}");
        assert!((86..=89).contains(&center[1]), "{center:?}");
        assert_eq!(center[3], 255);
    }
    #[test]
    fn gradients_paths_and_clipping_work_at_thumbnail_resolution() {
        let data = svg(
            r##"<defs><linearGradient id="g"><stop stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient><clipPath id="c"><rect width="20" height="20"/></clipPath></defs><path d="M0 0H40V20H0Z" fill="url(#g)" clip-path="url(#c)"/>"##,
        );
        let image = decode(data.as_bytes(), 40, 20).unwrap();
        assert!(image.get_pixel(2, 10)[0] > 200);
        assert!(image.get_pixel(18, 10)[2] > 100);
        assert_eq!(image.get_pixel(30, 10), &Rgba([176, 176, 176, 255]));
    }
    #[test]
    fn bounds_complexity_and_rejects_resource_expansion() {
        for body in [
            r#"<image href="file:///not-opened.png"/>"#,
            r#"<image href="data:image/svg+xml;base64,AAAA"/>"#,
            "<text>needs fonts</text>",
            "<filter/>",
            "<use/>",
            "<pattern/>",
            "<mask/>",
            "<marker/>",
        ] {
            assert!(decode(svg(body).as_bytes(), 40, 40).is_err(), "{body}");
        }
        assert!(decode(&vec![b' '; INPUT_LIMIT + 1], 40, 40).is_err());
        assert!(decode(svg(&"<g/>".repeat(NODE_LIMIT as usize)).as_bytes(), 40, 40).is_err());
        let deep = format!("{}{}", "<g>".repeat(33), "</g>".repeat(33));
        assert!(decode(svg(&deep).as_bytes(), 40, 40).is_err());
        assert!(
            decode(
                b"<!DOCTYPE svg [<!ENTITY x 'expanded'>]><svg>&x;</svg>",
                40,
                40
            )
            .is_err()
        );
    }
    #[test]
    fn huge_native_canvas_does_not_allocate_at_native_size() {
        let data = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10000000" height="10000000"><rect width="10000000" height="10000000" fill="red"/></svg>"#;
        let image = decode(data.as_bytes(), 24, 17).unwrap();
        assert_eq!(image.dimensions(), (24, 17));
        assert_eq!(image.get_pixel(12, 8), &Rgba([255, 0, 0, 255]));
    }
}
