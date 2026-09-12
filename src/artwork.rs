//! Small built-in vector shapes rasterized for Kitty. No fonts, assets, cache
//! lookups or decoder jobs are needed to represent folders and failed previews.
use image::{Rgba, RgbaImage, imageops};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Folder,
    File,
    Image,
    Video,
    Audio,
    Archive,
    Code,
    Config,
    Link,
    Special,
    Error,
}

pub fn render(icon: Icon, width: u32, height: u32, color: bool) -> RgbaImage {
    let mut art = Canvas(RgbaImage::new(100, 80));
    let fill = if !color {
        [175, 183, 196, 255]
    } else {
        match icon {
            Icon::Folder => [235, 182, 77, 255],
            Icon::Error => [220, 79, 85, 255],
            Icon::Image | Icon::Video | Icon::Audio => [160, 128, 213, 255],
            Icon::Archive => [209, 142, 77, 255],
            Icon::Code | Icon::Link => [84, 164, 205, 255],
            Icon::Config | Icon::Special => [114, 171, 155, 255],
            Icon::File => [153, 165, 185, 255],
        }
    };
    let dark = [
        (u16::from(fill[0]) * 3 / 4) as u8,
        (u16::from(fill[1]) * 3 / 4) as u8,
        (u16::from(fill[2]) * 3 / 4) as u8,
        255,
    ];
    let ink = [250, 250, 252, 255];
    if icon == Icon::Folder {
        art.polygon(
            &[(6, 17), (37, 17), (46, 26), (94, 26), (94, 69), (6, 69)],
            dark,
        );
        art.rect(6, 31, 94, 72, fill);
        art.line((12, 36), (88, 36), 2, ink);
    } else {
        art.polygon(&[(24, 5), (61, 5), (78, 22), (78, 75), (24, 75)], fill);
        art.polygon(&[(61, 5), (61, 22), (78, 22)], dark);
        match icon {
            Icon::Image => {
                art.circle(59, 34, 5, ink);
                art.polygon(&[(31, 61), (42, 43), (53, 56), (61, 48), (71, 61)], ink);
            }
            Icon::Video => art.polygon(&[(42, 32), (42, 61), (65, 47)], ink),
            Icon::Audio => {
                art.line((53, 33), (53, 57), 4, ink);
                art.line((53, 33), (65, 29), 4, ink);
                art.circle(46, 58, 7, ink);
            }
            Icon::Archive => {
                for y in (25..65).step_by(8) {
                    art.rect(47, y, 52, y + 4, ink);
                }
                art.rect(44, 60, 55, 68, dark);
            }
            Icon::Code => {
                art.line((44, 35), (35, 46), 3, ink);
                art.line((35, 46), (44, 57), 3, ink);
                art.line((58, 35), (67, 46), 3, ink);
                art.line((67, 46), (58, 57), 3, ink);
            }
            Icon::Error => {
                art.rect(48, 29, 54, 51, ink);
                art.circle(51, 61, 4, ink);
            }
            Icon::Link => {
                art.line((35, 59), (64, 34), 4, ink);
                art.line((49, 34), (64, 34), 4, ink);
                art.line((64, 34), (64, 49), 4, ink);
            }
            Icon::Special | Icon::Config => {
                art.circle(51, 47, 15, ink);
                art.circle(51, 47, 7, fill);
                for (x, y) in [(51, 27), (51, 67), (31, 47), (71, 47)] {
                    art.line((51, 47), (x, y), 3, ink);
                }
                art.circle(51, 47, 6, fill);
            }
            _ => {
                for y in [35, 45, 55, 65] {
                    art.line((34, y), (65, y), 2, ink);
                }
            }
        }
    }
    let scale = (width as f64 / 100.0).min(height as f64 / 80.0);
    let w = (100.0 * scale).round().max(1.0) as u32;
    let h = (80.0 * scale).round().max(1.0) as u32;
    let scaled = imageops::resize(&art.0, w, h, imageops::FilterType::Triangle);
    let mut result = RgbaImage::new(width, height);
    imageops::overlay(
        &mut result,
        &scaled,
        i64::from((width - w) / 2),
        i64::from((height - h) / 2),
    );
    result
}

struct Canvas(RgbaImage);
impl Canvas {
    fn rect(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: [u8; 4]) {
        // Integer rectangle edges cover exactly these pixel centers; no
        // point-in-polygon test is needed for the common solid rectangles.
        for y in y1.max(0)..y2.min(80) {
            for x in x1.max(0)..x2.min(100) {
                self.0.put_pixel(x as u32, y as u32, Rgba(color));
            }
        }
    }
    fn polygon<const N: usize>(&mut self, points: &[(i32, i32); N], color: [u8; 4]) {
        let y1 = points.iter().map(|p| p.1).min().unwrap_or(0).max(0);
        let y2 = points.iter().map(|p| p.1).max().unwrap_or(0).min(80);
        // Edge crossings depend only on the scanline. Compute them once, then
        // fill alternating spans using the same pixel-center inclusion rule.
        // All built-in shapes have at most six vertices; scratch stays on stack.
        let mut crossings = [0.0_f32; N];
        for y in y1..y2 {
            let py = y as f32 + 0.5;
            let mut count = 0;
            for i in 0..N {
                let (ax, ay) = points[i];
                let (bx, by) = points[(i + 1) % N];
                let (ax, ay, bx, by) = (ax as f32, ay as f32, bx as f32, by as f32);
                if (ay > py) != (by > py) {
                    crossings[count] = (bx - ax) * (py - ay) / (by - ay) + ax;
                    count += 1;
                }
            }
            crossings[..count].sort_unstable_by(f32::total_cmp);
            for pair in crossings[..count].as_chunks::<2>().0 {
                let x1 = (pair[0] - 0.5).ceil().clamp(0.0, 100.0) as i32;
                let x2 = (pair[1] - 0.5).ceil().clamp(0.0, 100.0) as i32;
                self.rect(x1, y, x2, y + 1, color);
            }
        }
    }
    fn circle(&mut self, cx: i32, cy: i32, r: i32, color: [u8; 4]) {
        for y in (cy - r).max(0)..=(cy + r).min(79) {
            for x in (cx - r).max(0)..=(cx + r).min(99) {
                if (x - cx).pow(2) + (y - cy).pow(2) <= r * r {
                    self.0.put_pixel(x as u32, y as u32, Rgba(color));
                }
            }
        }
    }
    fn line(&mut self, a: (i32, i32), b: (i32, i32), r: i32, color: [u8; 4]) {
        let steps = (a.0 - b.0).abs().max((a.1 - b.1).abs()).max(1);
        for i in 0..=steps {
            self.circle(
                a.0 + (b.0 - a.0) * i / steps,
                a.1 + (b.1 - a.1) * i / steps,
                r,
                color,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_polygon_matches_reference<const N: usize>(points: [(i32, i32); N]) {
        let color = [17, 81, 213, 255];
        let mut actual = Canvas(RgbaImage::new(100, 80));
        actual.polygon(&points, color);
        let expected = RgbaImage::from_fn(100, 80, |x, y| {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let mut inside = false;
            for i in 0..N {
                let (ax, ay) = points[i];
                let (bx, by) = points[(i + 1) % N];
                let (ax, ay, bx, by) = (ax as f32, ay as f32, bx as f32, by as f32);
                if (ay > py) != (by > py) && px < (bx - ax) * (py - ay) / (by - ay) + ax {
                    inside = !inside;
                }
            }
            Rgba(if inside { color } else { [0; 4] })
        });
        assert_eq!(actual.0, expected, "polygon {points:?}");
    }

    #[test]
    fn scanline_spans_preserve_every_source_pixel() {
        // Identical source canvases preserve every downstream thumbnail size.
        assert_polygon_matches_reference([
            (6, 17),
            (37, 17),
            (46, 26),
            (94, 26),
            (94, 69),
            (6, 69),
        ]);
        assert_polygon_matches_reference([(24, 5), (61, 5), (78, 22), (78, 75), (24, 75)]);
        assert_polygon_matches_reference([(61, 5), (61, 22), (78, 22)]);
        assert_polygon_matches_reference([(31, 61), (42, 43), (53, 56), (61, 48), (71, 61)]);
        assert_polygon_matches_reference([(42, 32), (42, 61), (65, 47)]);
        // Pixel-center diagonal crossings, concavity, reversed winding, clipping,
        // repeated vertices, self-intersections and zero-area shapes.
        for points in [
            [(0, 0), (80, 80), (0, 80), (80, 0)],
            [(-20, -20), (120, -20), (120, 100), (-20, 100)],
            [(20, 10), (50, 70), (50, 70), (80, 10)],
            [(5, 10), (30, 10), (60, 10), (90, 10)],
            [(10, 10); 4],
            [(0, 0), (100, 80), (50, 40), (0, 80)],
        ] {
            assert_polygon_matches_reference(points);
            let mut reverse = points;
            reverse.reverse();
            assert_polygon_matches_reference(reverse);
        }
        // Exercise varying edge slopes and multiple spans without random inputs
        // or a new test dependency. Coordinates also extend beyond the canvas.
        let mut state = 17_u32;
        for _ in 0..128 {
            let points = std::array::from_fn::<_, 6, _>(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let x = (state % 141) as i32 - 20;
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (x, (state % 121) as i32 - 20)
            });
            assert_polygon_matches_reference(points);
        }
        assert_polygon_matches_reference([]);
        assert_polygon_matches_reference([(20, 30)]);
        assert_polygon_matches_reference([(10, 10), (80, 70)]);
    }

    #[test]
    fn every_representation_is_visible_at_grid_and_inline_sizes() {
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
        for (w, h) in [(112, 51), (24, 17), (48, 48)] {
            for icon in icons {
                let image = render(icon, w, h, true);
                assert!(
                    image.pixels().filter(|p| p[3] > 64).count() > (w * h / 10) as usize,
                    "{icon:?}"
                );
                assert_eq!(image.get_pixel(0, 0)[3], 0);
            }
            assert_ne!(
                render(Icon::Error, w, h, true),
                render(Icon::File, w, h, true)
            );
        }
    }
}
