// SPDX-License-Identifier: GPL-3.0-or-later
//! Text for the widget kit: wrapping a string to a width in font pixels, and drawing the wrapped
//! lines from the game's bitmap fonts at a whole-number enlargement. Everything here is pure, so
//! the wrapping and the sizes are unit tested without a toolkit.

use crate::art::Image;
use crate::view::font::BitmapFont;

/// Which of the two kit fonts a piece of text uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    /// Ark Pixel, 10 px glyphs on a 14 px line: paragraphs, settings rows, the rules.
    Body,
    /// Press Start 2P, 8 px glyphs: titles, headings and button labels, as on Android.
    Display,
}

impl Face {
    /// The distance from one line's top to the next, in font pixels. The body font's own line
    /// box is already roomy; the display font gets three pixels of leading, as PixelUi gave it.
    pub fn pitch(self, font: &BitmapFont) -> i32 {
        match self {
            Face::Body => font.height(),
            Face::Display => font.height() + 3,
        }
    }
}

/// Where a line sits across the width it was wrapped to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
}

/// The characters the fonts lack that the app's texts use, drawn as the nearest ones they have.
/// Only the drawing changes: a screen reader still gets the text as written.
pub fn printable(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            'å' => out.push('a'),
            'Å' => out.push('A'),
            '®' => out.push_str("(R)"),
            '©' => out.push_str("(C)"),
            '§' => out.push_str("S."),
            '•' => out.push('*'),
            '\u{2013}' | '\u{2014}' => out.push('-'),
            '\u{2026}' => out.push_str("..."),
            '\u{00A0}' => out.push(' '),
            other => out.push(other),
        }
    }
    out
}

/// Breaks `text` into lines no wider than `max` font pixels of ink. Explicit newlines start new
/// lines, words are kept whole where they fit, and a word wider than the whole line is cut
/// between letters. An empty text is one empty line, so it still takes a line's height.
pub fn wrap(font: &BitmapFont, text: &str, max: i32) -> Vec<String> {
    let text = printable(text);
    let max = max.max(1);
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split(' ').filter(|w| !w.is_empty()) {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if font.ink_width(&candidate) <= max {
                line = candidate;
                continue;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            if font.ink_width(word) <= max {
                line = word.to_string();
                continue;
            }
            // A word longer than a whole line: as many letters as fit, then the rest below.
            for c in word.chars() {
                let mut next = line.clone();
                next.push(c);
                if !line.is_empty() && font.ink_width(&next) > max {
                    lines.push(std::mem::replace(&mut line, c.to_string()));
                } else {
                    line = next;
                }
            }
        }
        lines.push(line);
    }
    lines
}

/// A block of text laid out for a width: its lines and its size in art pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub face: Face,
    /// Art pixels per font pixel: 1, or more for large text.
    pub mult: i32,
    pub lines: Vec<String>,
    pub align: Align,
    /// The width the lines were wrapped to, in art pixels.
    pub width: i32,
    /// The pitch times the number of lines, in art pixels.
    pub height: i32,
    pitch: i32,
}

impl Block {
    /// Lays `text` out in `face` at `mult` for `width` art pixels.
    pub fn new(
        font: &BitmapFont,
        face: Face,
        mult: i32,
        text: &str,
        width: i32,
        align: Align,
    ) -> Block {
        let mult = mult.max(1);
        let lines = wrap(font, text, width / mult);
        let pitch = face.pitch(font) * mult;
        // The display font's leading falls below its last line too; the last line only needs
        // its own glyphs, so a title is not padded with space it does not use.
        let tail = match face {
            Face::Body => 0,
            Face::Display => 3 * mult,
        };
        let height = pitch * lines.len() as i32 - tail;
        Block {
            face,
            mult,
            lines,
            align,
            width,
            height,
            pitch,
        }
    }

    /// The widest line's ink, in art pixels.
    pub fn ink_width(&self, font: &BitmapFont) -> i32 {
        self.lines
            .iter()
            .map(|l| font.ink_width(l) * self.mult)
            .max()
            .unwrap_or(0)
    }

    /// Draws the block with its top left at (x, y), in `argb`.
    pub fn draw(&self, font: &BitmapFont, img: &mut Image, x: i32, y: i32, argb: u32) {
        for (i, line) in self.lines.iter().enumerate() {
            let w = font.ink_width(line) * self.mult;
            let lx = match self.align {
                Align::Start => x,
                Align::Center => x + (self.width - w) / 2,
                Align::End => x + self.width - w,
            };
            draw_line(
                font,
                img,
                line,
                lx,
                y + i as i32 * self.pitch,
                self.mult,
                argb,
            );
        }
    }
}

/// One line of text at `mult` art pixels per font pixel. At 1 it is drawn straight onto the
/// image; larger, it is drawn once at font size and copied up with nearest-neighbor sampling,
/// so every font pixel is an exact square.
pub fn draw_line(
    font: &BitmapFont,
    img: &mut Image,
    s: &str,
    x: i32,
    y: i32,
    mult: i32,
    argb: u32,
) {
    if mult <= 1 {
        font.draw(img, s, x, y, argb);
        return;
    }
    let w = font.width(s).max(1) as u32;
    let mut small = Image::new(w, font.height().max(1) as u32);
    font.draw(&mut small, s, 0, 0, argb);
    img.sprite(&small.upscale(mult as u32), x, y, 255);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::assets::{FONT_BODY, FONT_BODY_TXT, FONT_DISPLAY, FONT_DISPLAY_TXT};

    fn body() -> BitmapFont {
        BitmapFont::parse(FONT_BODY, FONT_BODY_TXT).unwrap()
    }

    fn display() -> BitmapFont {
        BitmapFont::parse(FONT_DISPLAY, FONT_DISPLAY_TXT).unwrap()
    }

    #[test]
    fn wraps_at_word_boundaries_within_the_width() {
        let font = body();
        let text = "You get three rolls each turn. Tap on dice to save them, then roll the rest.";
        let lines = wrap(&font, text, 120);
        assert!(lines.len() > 1);
        for l in &lines {
            assert!(font.ink_width(l) <= 120, "{l:?} is too wide");
            assert!(!l.starts_with(' ') && !l.ends_with(' '));
        }
        assert_eq!(lines.join(" "), text);
    }

    #[test]
    fn keeps_explicit_breaks_and_empty_lines() {
        let font = body();
        assert_eq!(wrap(&font, "a\n\nb", 200), vec!["a", "", "b"]);
        assert_eq!(wrap(&font, "", 200), vec![""]);
    }

    #[test]
    fn cuts_a_word_longer_than_the_line() {
        let font = display();
        // The display font is 8 px a cell, so 4 letters fit in 32
        let lines = wrap(&font, "abcdefghij", 32);
        assert_eq!(lines, vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn missing_letters_become_their_nearest() {
        assert_eq!(printable("Håskjold CATAN®"), "Haskjold CATAN(R)");
        assert_eq!(printable("§ 102"), "S. 102");
    }

    #[test]
    fn block_size_follows_the_enlargement() {
        let font = body();
        let one = Block::new(&font, Face::Body, 1, "Sound effects", 200, Align::Start);
        assert_eq!(one.lines.len(), 1);
        assert_eq!(one.height, 14);
        let two = Block::new(&font, Face::Body, 2, "Sound effects", 200, Align::Start);
        assert_eq!(two.height, 28);
        assert_eq!(two.ink_width(&font), one.ink_width(&font) * 2);
        // Display text drops the leading under its last line
        let d = display();
        let title = Block::new(&d, Face::Display, 1, "Menu", 200, Align::Center);
        assert_eq!(title.height, 8);
        let two_lines = Block::new(&d, Face::Display, 1, "Top Scores", 48, Align::Center);
        assert_eq!(two_lines.lines.len(), 2);
        assert_eq!(two_lines.height, 11 + 8);
    }

    #[test]
    fn enlarged_text_is_made_of_whole_squares() {
        let font = body();
        let mut img = Image::new(60, 40);
        draw_line(&font, &mut img, "1", 0, 0, 3, 0xFF000000);
        // Every inked 3 x 3 cell is all ink or all clear
        for cy in 0..13 {
            for cx in 0..20 {
                let first = img.argb_at(cx * 3, cy * 3);
                for dy in 0..3 {
                    for dx in 0..3 {
                        assert_eq!(img.argb_at(cx * 3 + dx, cy * 3 + dy), first);
                    }
                }
            }
        }
        assert!(img.to_argb().contains(&0xFF000000));
    }

    #[test]
    fn centered_lines_sit_in_the_middle() {
        let font = display();
        let b = Block::new(&font, Face::Display, 1, "OK", 40, Align::Center);
        let mut img = Image::new(40, 8);
        b.draw(&font, &mut img, 0, 0, 0xFF000000);
        let left = (0..40)
            .find(|&x| (0..8).any(|y| img.argb_at(x, y) != 0))
            .unwrap();
        let right = (0..40)
            .rev()
            .find(|&x| (0..8).any(|y| img.argb_at(x, y) != 0))
            .unwrap();
        assert!((left - (39 - right)).abs() <= 1, "{left} {right}");
    }
}
