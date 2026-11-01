// SPDX-License-Identifier: GPL-3.0-or-later
//! Text from a baked glyph atlas, drawn onto the art framebuffer at one art pixel per font
//! pixel: a port of the Android app's BitmapFont. The atlas and metrics come from
//! tools/bake_font.py: a white, alpha only atlas, and one "codepoint x y w h advance" line
//! per glyph after a "height baseline" first line.

use std::collections::HashMap;

use crate::art::{Image, rgba};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Glyph {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    advance: i32,
    /// One past the last inked column of the cell, so text can be centred on its ink.
    ink: i32,
}

#[derive(Clone, Debug)]
pub struct BitmapFont {
    atlas: Image,
    glyphs: HashMap<u32, Glyph>,
    height: i32,
}

impl BitmapFont {
    /// Reads an atlas PNG and its metrics text.
    pub fn parse(atlas_png: &[u8], metrics: &str) -> Result<BitmapFont, String> {
        let atlas = Image::from_png(atlas_png)?;
        let mut lines = metrics.lines();
        let height = lines
            .next()
            .and_then(|l| l.split_whitespace().next())
            .and_then(|h| h.parse().ok())
            .ok_or("font metrics have no height line")?;
        let mut glyphs = HashMap::new();
        for line in lines {
            let f: Vec<i32> = line
                .split_whitespace()
                .filter_map(|n| n.parse().ok())
                .collect();
            if f.len() < 6 {
                continue;
            }
            let mut g = Glyph {
                x: f[1],
                y: f[2],
                w: f[3],
                h: f[4],
                advance: f[5],
                ink: 0,
            };
            g.ink = (0..g.w)
                .rev()
                .find(|&cx| (0..g.h).any(|cy| atlas.argb_at(g.x + cx, g.y + cy) >> 24 != 0))
                .map_or(0, |cx| cx + 1);
            glyphs.insert(f[0] as u32, g);
        }
        Ok(BitmapFont {
            atlas,
            glyphs,
            height,
        })
    }

    /// The glyphs a string draws as. Fluent wraps every placeable in Unicode isolation marks,
    /// which have no glyph and no width, so they are dropped; the typographic quotes and spaces
    /// some translations use are drawn as the plain ones the atlas has. Anything else the atlas
    /// lacks is a "?", as on Android.
    fn glyphs_of<'a>(&'a self, s: &'a str) -> impl Iterator<Item = Glyph> + 'a {
        s.chars().filter_map(move |c| {
            let c = match c {
                '\u{2066}'..='\u{2069}' | '\u{200E}' | '\u{200F}' | '\u{200B}' => return None,
                '\u{2018}' | '\u{2019}' => '\'',
                '\u{201C}' | '\u{201D}' => '"',
                '\u{202F}' | '\u{2009}' => ' ',
                other => other,
            };
            self.glyphs
                .get(&(c as u32))
                .or_else(|| self.glyphs.get(&('?' as u32)))
                .copied()
        })
    }

    /// Pen advance of the whole string.
    pub fn width(&self, s: &str) -> i32 {
        self.glyphs_of(s).map(|g| g.advance).sum()
    }

    /// Width up to the last inked pixel, without the final glyph's trailing space.
    pub fn ink_width(&self, s: &str) -> i32 {
        let (mut pen, mut ink) = (0, 0);
        for g in self.glyphs_of(s) {
            if g.ink > 0 {
                ink = pen + g.ink;
            }
            pen += g.advance;
        }
        ink
    }

    /// The line box's height.
    pub fn height(&self) -> i32 {
        self.height
    }

    /// (x, y) is the top left of the line box. The atlas's alpha is the coverage, in the
    /// colour asked for.
    pub fn draw(&self, art: &mut Image, s: &str, x: i32, y: i32, argb: u32) {
        let color = rgba(argb);
        let mut pen = x;
        for g in self.glyphs_of(s) {
            for cy in 0..g.h {
                for cx in 0..g.w {
                    let a = (self.atlas.argb_at(g.x + cx, g.y + cy) >> 24) as u8;
                    if a == 0 {
                        continue;
                    }
                    let alpha = (color[3] as u32 * a as u32 / 255) as u8;
                    art.fill(
                        pen + cx,
                        y + cy,
                        1,
                        1,
                        [color[0], color[1], color[2], alpha],
                    );
                }
            }
            pen += g.advance;
        }
    }

    /// The string with a one pixel outline all round, for text that sits on terrain.
    pub fn draw_outlined(&self, art: &mut Image, s: &str, x: i32, y: i32, argb: u32, outline: u32) {
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx != 0 || dy != 0 {
                    self.draw(art, s, x + dx, y + dy, outline);
                }
            }
        }
        self.draw(art, s, x, y, argb);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::assets::{FONT_BODY, FONT_BODY_TXT, FONT_DISPLAY, FONT_DISPLAY_TXT};

    fn body() -> BitmapFont {
        BitmapFont::parse(FONT_BODY, FONT_BODY_TXT).unwrap()
    }

    #[test]
    fn reads_the_metrics() {
        let font = body();
        assert_eq!(font.height(), 14);
        // A space advances but has no ink
        assert_eq!(font.width(" "), 5);
        assert_eq!(font.ink_width(" "), 0);
        assert!(font.ink_width("A") > 0 && font.ink_width("A") <= font.width("A"));
    }

    #[test]
    fn ink_width_leaves_off_the_trailing_space() {
        let font = body();
        let w = font.ink_width("Costs");
        assert!(w < font.width("Costs"));
        assert_eq!(font.ink_width("Costs "), w);
        // The display font is monospaced, 8 px a cell
        let display = BitmapFont::parse(FONT_DISPLAY, FONT_DISPLAY_TXT).unwrap();
        assert_eq!(display.width("Play again"), 80);
    }

    #[test]
    fn isolation_marks_vanish_and_unknown_letters_are_questions() {
        let font = body();
        assert_eq!(font.width("\u{2068}12\u{2069}"), font.width("12"));
        assert_eq!(font.width("\u{4E00}"), font.width("?"));
        assert_eq!(font.width("King\u{2019}s"), font.width("King's"));
    }

    #[test]
    fn draws_in_the_colour_asked_for() {
        let font = body();
        let mut img = Image::new(20, 14);
        font.draw(&mut img, "1", 0, 0, 0xFFFF0000);
        let red = img
            .to_argb()
            .into_iter()
            .filter(|&p| p == 0xFFFF0000)
            .count();
        assert!(red > 3, "the digit drew {red} pixels");
        assert!(img.to_argb().iter().all(|&p| p == 0 || p == 0xFFFF0000));
    }
}
