//! A PERSONAL FORK OF 'drawille' VERSION 0.3.0
//! WITH ANSI COLOR SUPPORT
//!
//! `drawille` – a terminal graphics library for Rust, based on the Python library
//! [drawille](https://github.com/asciimoo/drawille).
//!
//! This crate provides an interface for utilising Braille characters to draw a picture to a
//! terminal, allowing for much smaller pixels but losing proper colour support.
//!
//! # Example
//!
//! ```
//! extern crate drawille;
//!
//! use drawille::Canvas;
//!
//! fn main() {
//!     let mut canvas = Canvas::new(10, 10);
//!     canvas.set(5, 4);
//!     canvas.line(2, 2, 8, 8);
//!     assert_eq!(canvas.frame(), [
//! " ⢄    ",
//! "  ⠙⢄  ",
//! "    ⠁ "].join("\n"));
//! }
//! ```
use std::char;
use std::cmp;

use fnv::FnvHashMap;
pub use owo_colors::AnsiColors as PixelColor;
use owo_colors::OwoColorize;

// extern crate colored;
// pub use colored::Color as PixelColor;
// use colored::Colorize;

static PIXEL_MAP: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];

/// A canvas object that can be used to draw to the terminal using Braille characters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Canvas {
    chars: FnvHashMap<(u16, u16), (u8, char, bool, PixelColor)>,
    width: u16,
    height: u16,
}

impl Canvas {
    /// Creates a new `Canvas` with the given width and height.
    ///
    /// Note that the `Canvas` can still draw outside the given dimensions (expanding the canvas)
    /// if a pixel is set outside the dimensions.
    pub fn new(width: u32, height: u32) -> Canvas {
        Canvas {
            chars: FnvHashMap::default(),
            width: (width / 2) as u16,
            height: (height / 4) as u16,
        }
    }

    /// Clears the canvas.
    pub fn clear(&mut self) {
        self.chars.clear();
    }

    /// Sets a pixel at the specified coordinates.
    pub fn set(&mut self, x: u32, y: u32) {
        let (row, col) = ((x / 2) as u16, (y / 4) as u16);
        let a = self
            .chars
            .entry((row, col))
            .or_insert((0, ' ', false, PixelColor::White));
        a.0 |= PIXEL_MAP[y as usize % 4][x as usize % 2];
        a.1 = ' ';
        a.2 = false;
        a.3 = PixelColor::White;
    }

    /// Sets a pixel at the specified coordinates.
    /// specifying the color of the braille char
    pub fn set_colored(&mut self, x: u32, y: u32, color: PixelColor) {
        let (row, col) = ((x / 2) as u16, (y / 4) as u16);
        let a = self
            .chars
            .entry((row, col))
            .or_insert((0, ' ', false, PixelColor::White));
        a.0 |= PIXEL_MAP[y as usize % 4][x as usize % 2];
        a.1 = ' ';
        a.2 = true;
        a.3 = color;
    }

    /// Sets a letter at the specified coordinates.
    pub fn set_char(&mut self, x: u32, y: u32, c: char) {
        let (row, col) = ((x / 2) as u16, (y / 4) as u16);
        let a = self
            .chars
            .entry((row, col))
            .or_insert((0, ' ', false, PixelColor::White));
        a.0 = 0;
        a.1 = c;
        a.2 = false;
        a.3 = PixelColor::White;
    }

    /// Draws text at the specified coordinates (top-left of the text) up to max_width length
    pub fn text(&mut self, x: u32, y: u32, max_width: u32, text: &str) {
        for (i, c) in text.chars().enumerate() {
            let w = i as u32 * 2;
            if w > max_width {
                return;
            }
            self.set_char(x + w, y, c);
        }
    }

    /// Deletes a pixel at the specified coordinates.
    pub fn unset(&mut self, x: u32, y: u32) {
        let (row, col) = ((x / 2) as u16, (y / 4) as u16);
        let a = self
            .chars
            .entry((row, col))
            .or_insert((0, ' ', false, PixelColor::White));
        a.0 &= !PIXEL_MAP[y as usize % 4][x as usize % 2];
    }

    /// Toggles a pixel at the specified coordinates.
    pub fn toggle(&mut self, x: u32, y: u32) {
        let (row, col) = ((x / 2) as u16, (y / 4) as u16);
        let a = self
            .chars
            .entry((row, col))
            .or_insert((0, ' ', false, PixelColor::White));
        a.0 ^= PIXEL_MAP[y as usize % 4][x as usize % 2];
    }

    /// Detects whether the pixel at the given coordinates is set.
    pub fn get(&self, x: u32, y: u32) -> bool {
        let (row, col) = ((x / 2) as u16, (y / 4) as u16);
        self.chars.get(&(row, col)).map_or(false, |a| {
            let dot_index = PIXEL_MAP[y as usize % 4][x as usize % 2];
            a.0 & dot_index != 0
        })
    }

    /// Returns a `Vec` of each row of the `Canvas`.
    ///
    /// Note that each row is actually four pixels high due to the fact that a single Braille
    /// character spans two by four pixels.
    pub fn rows(&self) -> Vec<String> {
        let mut maxrow = self.width;
        let mut maxcol = self.height;
        for &(x, y) in self.chars.keys() {
            if x > maxrow {
                maxrow = x;
            }
            if y > maxcol {
                maxcol = y;
            }
        }

        let mut result = Vec::with_capacity(maxcol as usize + 1);
        for y in 0..=maxcol {
            let mut row = String::with_capacity(maxrow as usize + 1);
            for x in 0..=maxrow {
                let cell =
                    self.chars
                        .get(&(x, y))
                        .cloned()
                        .unwrap_or((0, ' ', false, PixelColor::White));
                match cell {
                    (0, _, _, _) => row.push(cell.1),
                    (_, _, false, _) => row.push(char::from_u32(0x2800 + cell.0 as u32).unwrap()),
                    (_, _, true, _) => {
                        row = format!(
                            "{0}{1}",
                            row,
                            String::from(char::from_u32(0x2800 + cell.0 as u32).unwrap())
                                .color(cell.3)
                        )
                    }
                };
            }
            result.push(row);
        }
        result
    }

    /// Draws the canvas to a `String` and returns it.
    pub fn frame(&self) -> String {
        self.rows().join("\n")
    }

    /// Draws a line from `(x1, y1)` to `(x2, y2)` onto the `Canvas`.
    pub fn line(&mut self, x1: u32, y1: u32, x2: u32, y2: u32) {
        let xdiff = cmp::max(x1, x2) - cmp::min(x1, x2);
        let ydiff = cmp::max(y1, y2) - cmp::min(y1, y2);
        let xdir = if x1 <= x2 { 1 } else { -1 };
        let ydir = if y1 <= y2 { 1 } else { -1 };

        let r = cmp::max(xdiff, ydiff);

        for i in 0..=r {
            let mut x = x1 as i32;
            let mut y = y1 as i32;

            if ydiff != 0 {
                y += ((i * ydiff) / r) as i32 * ydir;
            }
            if xdiff != 0 {
                x += ((i * xdiff) / r) as i32 * xdir;
            }

            self.set(x as u32, y as u32);
        }
    }

    /// Draws a line from `(x1, y1)` to `(x2, y2)` onto the `Canvas`
    /// specifying the color of the line
    pub fn line_colored(&mut self, x1: u32, y1: u32, x2: u32, y2: u32, color: PixelColor) {
        let xdiff = cmp::max(x1, x2) - cmp::min(x1, x2);
        let ydiff = cmp::max(y1, y2) - cmp::min(y1, y2);
        let xdir = if x1 <= x2 { 1 } else { -1 };
        let ydir = if y1 <= y2 { 1 } else { -1 };

        let r = cmp::max(xdiff, ydiff);

        for i in 0..=r {
            let mut x = x1 as i32;
            let mut y = y1 as i32;

            if ydiff != 0 {
                y += ((i * ydiff) / r) as i32 * ydir;
            }
            if xdiff != 0 {
                x += ((i * xdiff) / r) as i32 * xdir;
            }

            self.set_colored(x as u32, y as u32, color);
        }
    }
}
