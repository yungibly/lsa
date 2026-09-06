//! Kitty ownership for the browser, separate from anonymous inline placements.
use base64::{Engine, engine::general_purpose::STANDARD};
use image::RgbaImage;
use std::{
    collections::hash_map::RandomState,
    hash::BuildHasher,
    io::{self, Write},
};

pub const MAX_VISIBLE: usize = 32;
pub const IMAGE_ROWS: usize = 5;
pub const TILE_ROWS: usize = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometry {
    pub columns: usize,
    pub tile: usize,
    pub lines: usize,
    pub width: u32,
    pub height: u32,
}

impl Geometry {
    pub fn new(term: &crate::terminal::Terminal) -> Option<Self> {
        if !term.kitty || term.cols < 12 || term.rows < 10 {
            return None;
        }
        let inline = crate::grid::Geometry::new(term)?;
        Some(Self {
            columns: inline.columns,
            tile: inline.tile,
            lines: ((term.rows.min(256) - 3) / TILE_ROWS).min(MAX_VISIBLE / inline.columns),
            width: inline.width,
            height: inline.height,
        })
    }

    pub fn capacity(self) -> usize {
        self.columns * self.lines
    }

    pub fn position(self, offset: usize) -> (usize, usize) {
        (
            2 + offset / self.columns * TILE_ROWS,
            1 + offset % self.columns * self.tile,
        )
    }

    pub fn upload_bound(self) -> usize {
        upload_len(self.width, self.height, u32::MAX, self.tile - 2) + deletion(u32::MAX).len()
    }
}

fn header(width: u32, height: u32, number: u32, cols: usize, more: bool) -> String {
    // I creates a fresh image rather than replacing somebody else's image id.
    // All subsequent operations name this randomized number; p=1 replaces its
    // one placement when moving, q=2 suppresses replies, C=1 preserves the cursor.
    format!(
        "\x1b_Ga=T,t=d,f=32,I={number},p=1,s={width},v={height},c={cols},r={IMAGE_ROWS},C=1,q=2,m={};",
        u8::from(more)
    )
}

fn upload_len(width: u32, height: u32, number: u32, cols: usize) -> usize {
    let payload = (width as usize * height as usize * 4).div_ceil(3) * 4;
    let chunks = payload.div_ceil(4096);
    header(width, height, number, cols, chunks > 1).len()
        + payload
        + 2
        + (chunks - 1) * "\x1b_Gm=1,q=2;\x1b\\".len()
}

fn deletion(number: u32) -> String {
    format!("\x1b_Ga=d,d=N,I={number},q=2;\x1b\\")
}

struct Placement {
    number: u32,
    row: usize,
    col: usize,
}

pub struct Screen {
    placements: Vec<Placement>,
    remaining: usize,
    seed: RandomState,
    serial: u64,
}

impl Default for Screen {
    fn default() -> Self {
        Self {
            placements: Vec::new(),
            remaining: crate::preview::OUTPUT_LIMIT,
            seed: RandomState::new(),
            serial: 0,
        }
    }
}

impl Screen {
    pub fn remaining(&self) -> usize {
        self.remaining
    }

    pub fn upload(
        &mut self,
        out: &mut impl Write,
        image: &RgbaImage,
        geometry: Geometry,
        row: usize,
        col: usize,
    ) -> io::Result<Option<u32>> {
        if self.placements.len() >= MAX_VISIBLE {
            return Ok(None);
        }
        let number = loop {
            self.serial += 1;
            let number = self.seed.hash_one(self.serial) as u32;
            if number != 0 && !self.placements.iter().any(|p| p.number == number) {
                break number;
            }
        };
        let bytes = upload_len(image.width(), image.height(), number, geometry.tile - 2)
            + deletion(number).len();
        if bytes > self.remaining {
            return Ok(None);
        }
        self.remaining -= bytes; // Reserve cleanup even if output later fails.
        self.placements.push(Placement { number, row, col });
        write!(out, "\x1b[{row};{col}H")?;
        let encoded = STANDARD.encode(image.as_raw());
        let mut chunks = encoded.as_bytes().chunks(4096).peekable();
        let mut first = true;
        while let Some(chunk) = chunks.next() {
            let more = chunks.peek().is_some();
            if first {
                out.write_all(
                    header(
                        image.width(),
                        image.height(),
                        number,
                        geometry.tile - 2,
                        more,
                    )
                    .as_bytes(),
                )?;
                first = false;
            } else {
                write!(out, "\x1b_Gm={},q=2;", u8::from(more))?;
            }
            out.write_all(chunk)?;
            out.write_all(b"\x1b\\")?;
        }
        Ok(Some(number))
    }

    pub fn move_to(
        &mut self,
        out: &mut impl Write,
        number: u32,
        geometry: Geometry,
        row: usize,
        col: usize,
    ) -> io::Result<bool> {
        let Some(p) = self.placements.iter_mut().find(|p| p.number == number) else {
            return Ok(false);
        };
        if (p.row, p.col) == (row, col) {
            return Ok(true);
        }
        let command = format!(
            "\x1b_Ga=p,I={number},p=1,c={},r={IMAGE_ROWS},C=1,q=2;\x1b\\",
            geometry.tile - 2
        );
        if command.len() > self.remaining {
            return Ok(false);
        }
        self.remaining -= command.len();
        write!(out, "\x1b[{row};{col}H{command}")?;
        p.row = row;
        p.col = col;
        Ok(true)
    }

    pub fn remove(&mut self, out: &mut impl Write, number: u32) -> io::Result<()> {
        if let Some(i) = self.placements.iter().position(|p| p.number == number) {
            out.write_all(deletion(number).as_bytes())?;
            self.placements.swap_remove(i);
        }
        Ok(())
    }

    pub fn clear(&mut self, out: &mut impl Write) -> io::Result<()> {
        while let Some(p) = self.placements.last() {
            self.remove(out, p.number)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owned_upload_move_cleanup_and_exact_budget() {
        let mut screen = Screen::default();
        let g = Geometry {
            columns: 1,
            tile: 24,
            lines: 1,
            width: 176,
            height: 80,
        };
        let image = RgbaImage::new(g.width, g.height);
        let mut out = Vec::new();
        let number = screen.upload(&mut out, &image, g, 2, 1).unwrap().unwrap();
        let expected = upload_len(g.width, g.height, number, 22);
        assert_eq!(out.len(), expected + "\x1b[2;1H".len());
        assert!(g.upload_bound() >= expected + deletion(number).len());
        let before = out.len();
        assert!(screen.move_to(&mut out, number, g, 2, 1).unwrap());
        assert_eq!(before, out.len());
        assert!(screen.move_to(&mut out, number, g, 9, 1).unwrap());
        screen.remaining = 0;
        assert!(screen.upload(&mut out, &image, g, 2, 1).unwrap().is_none());
        assert!(!screen.move_to(&mut out, number, g, 2, 1).unwrap());
        screen.clear(&mut out).unwrap(); // Cleanup still works with no remaining budget.
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains(&format!("I={number},p=1")));
        assert!(text.ends_with(&deletion(number)));
        assert!(!text.contains("d=A") && !text.contains("d=I") && !text.contains(",i="));
    }
}
