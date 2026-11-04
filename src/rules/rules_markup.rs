// SPDX-License-Identifier: GPL-3.0-or-later
//! Reads the rules text into blocks the Rules screen lays out, and the whole-factor shrink
//! its pictures get.
//!
//! The format. One construct per line; a line is judged after trimming its ends.
//!
//! - plain text: running text. Consecutive plain lines run together as one paragraph (so a
//!   file may be wrapped by hand); a blank line ends it.
//! - "# Heading": a section heading, in the display font. Needs the space after "#".
//! - "- Item": a bullet with a hanging indent: wrapped lines stay inside the text.
//! - "[icon_name] text": a line that starts with a 12 px icon, the text beside it, wrapping
//!   inside its own column.
//! - "![name]" on a line of its own: the picture called name, centred, at the art scale with
//!   nearest neighbour.
//!
//! Names are letters, digits and underscores, with no ".png". There is no inline styling and
//! no nesting: a bullet is one line of text, and a line that starts with "[" but is not
//! "[name] text" is plain text.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A section heading, in the display font.
    Heading,
    /// Running text.
    Paragraph,
    /// A picture, centred. name holds its name.
    Image,
    /// A line with a hanging bullet.
    Bullet,
    /// A line that starts with a small picture. name is the picture, text the line.
    Icon,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub kind: Kind,
    pub name: Option<String>,
    pub text: Option<String>,
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.kind)?;
        if let Some(name) = &self.name {
            write!(f, "[{name}]")?;
        }
        if let Some(text) = &self.text {
            write!(f, " {text}")?;
        }
        Ok(())
    }
}

fn is_name(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// "![name]" on its own.
fn image(line: &str) -> Option<&str> {
    let name = line.strip_prefix("![")?.strip_suffix(']')?;
    is_name(name).then_some(name)
}

/// "[name] text": the name, then at least one white space character, then some text.
fn icon(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix('[')?;
    let close = rest.find(']')?;
    let name = &rest[..close];
    let after = &rest[close + 1..];
    if !is_name(name) || !after.starts_with(char::is_whitespace) {
        return None;
    }
    let text = after.trim_start();
    (!text.is_empty()).then_some((name, text))
}

/// Trims as Java's String.trim does: every character at or below a space.
fn trim(s: &str) -> &str {
    s.trim_matches(|c: char| c <= ' ')
}

pub fn parse(source: Option<&str>) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut paragraph = String::new();
    let text = source.unwrap_or("");
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);

    let normalised = text.replace("\r\n", "\n").replace('\r', "\n");
    for raw in normalised.split('\n') {
        let line = trim(raw);
        if line.is_empty() {
            flush(&mut blocks, &mut paragraph);
            continue;
        }
        if let Some(heading) = line.strip_prefix("# ") {
            flush(&mut blocks, &mut paragraph);
            blocks.push(Block {
                kind: Kind::Heading,
                name: None,
                text: Some(trim(heading).to_owned()),
            });
        } else if let Some(name) = image(line) {
            flush(&mut blocks, &mut paragraph);
            blocks.push(Block {
                kind: Kind::Image,
                name: Some(name.to_owned()),
                text: None,
            });
        } else if let Some(bullet) = line.strip_prefix("- ") {
            flush(&mut blocks, &mut paragraph);
            blocks.push(Block {
                kind: Kind::Bullet,
                name: None,
                text: Some(trim(bullet).to_owned()),
            });
        } else if let Some((name, text)) = icon(line) {
            flush(&mut blocks, &mut paragraph);
            blocks.push(Block {
                kind: Kind::Icon,
                name: Some(name.to_owned()),
                text: Some(trim(text).to_owned()),
            });
        } else {
            // Lines of one paragraph may be wrapped by hand in the file: they run together
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(line);
        }
    }
    flush(&mut blocks, &mut paragraph);
    blocks
}

fn flush(blocks: &mut Vec<Block>, paragraph: &mut String) {
    if !paragraph.is_empty() {
        blocks.push(Block {
            kind: Kind::Paragraph,
            name: None,
            text: Some(std::mem::take(paragraph)),
        });
    }
}

/// The whole factor a picture art_width wide is made smaller by to fit a column max_art
/// wide: 1 when it fits, else 2, 3 and so on, never a fraction.
pub fn shrink_factor(art_width: i32, max_art: i32) -> i32 {
    if max_art <= 0 || art_width <= max_art {
        return 1;
    }
    (art_width + max_art - 1) / max_art
}

/// Makes pixels (width x height, row by row) k times smaller each way by keeping the pixel at
/// the centre of every k x k block. The result is width / k by height / k; the few leftover
/// columns and rows are cropped evenly from both sides, so a picture is never smeared and
/// never given a fractional pixel.
pub fn shrink(pixels: &[u32], width: usize, height: usize, k: usize) -> Vec<u32> {
    if k <= 1 {
        return pixels.to_vec();
    }
    let w = width / k;
    let h = height / k;
    let off_x = (width - w * k) / 2;
    let off_y = (height - h * k) / 2;
    let mut out = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            out.push(pixels[(off_y + y * k + k / 2) * width + off_x + x * k + k / 2]);
        }
    }
    out
}
