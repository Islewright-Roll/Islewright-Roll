// SPDX-License-Identifier: GPL-3.0-or-later
//! A small JSON document type with a parser and two writers, compact and indented.
//!
//! serde_json is not a direct dependency of this crate, and the saved game and the scores
//! document both need to be read field by field with their own fallbacks (a missing key
//! takes a default, a wrong one is skipped), which is easier over a tree than through
//! derived types. Objects keep their keys in the order they were added, so a document is
//! written the way it was built and a test can look for an exact fragment of it.

use std::fmt::Write as _;

/// One JSON value.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    /// A number written without a fraction or an exponent.
    Int(i64),
    /// Any other number.
    Float(f64),
    Str(String),
    Array(Vec<Json>),
    /// Keys in insertion order. A key set twice keeps its first place.
    Object(Vec<(String, Json)>),
}

/// Why a document could not be read, or a field of it was not what it had to be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError(pub String);

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for JsonError {}

impl From<&str> for JsonError {
    fn from(s: &str) -> Self {
        JsonError(s.to_owned())
    }
}

impl From<String> for JsonError {
    fn from(s: String) -> Self {
        JsonError(s)
    }
}

impl Json {
    /// An empty object, to be filled with `put`.
    pub fn object() -> Json {
        Json::Object(Vec::new())
    }

    /// Parses a whole document. Anything after the value but white space is an error.
    pub fn parse(text: &str) -> Result<Json, JsonError> {
        let mut p = Parser {
            s: text.as_bytes(),
            at: 0,
        };
        p.skip_ws();
        let value = p.value(0)?;
        p.skip_ws();
        if p.at != p.s.len() {
            return Err(p.error("text after the end of the document"));
        }
        Ok(value)
    }

    /// Sets a key of an object, replacing its value if it is there already. Does nothing to
    /// anything but an object.
    pub fn put(&mut self, key: &str, value: impl Into<Json>) {
        if let Json::Object(entries) = self {
            let value = value.into();
            if let Some(entry) = entries.iter_mut().find(|(k, _)| k == key) {
                entry.1 = value;
            } else {
                entries.push((key.to_owned(), value));
            }
        }
    }

    /// Takes a key out of an object.
    pub fn remove(&mut self, key: &str) -> Option<Json> {
        if let Json::Object(entries) = self {
            let at = entries.iter().position(|(k, _)| k == key)?;
            return Some(entries.remove(at).1);
        }
        None
    }

    /// The value under a key of an object, or None for a missing key or a non object.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Json> {
        match self {
            Json::Object(entries) => entries.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// The keys of an object in order; none for anything else.
    pub fn keys(&self) -> Vec<&str> {
        match self {
            Json::Object(entries) => entries.iter().map(|(k, _)| k.as_str()).collect(),
            _ => Vec::new(),
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Json>> {
        match self {
            Json::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&Vec<(String, Json)>> {
        match self {
            Json::Object(entries) => Some(entries),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Whether this is a number of either kind (not a string that reads as one).
    pub fn is_number(&self) -> bool {
        matches!(self, Json::Int(_) | Json::Float(_))
    }

    /// A number as a whole number, a fraction cut off toward zero. A string holding a
    /// number counts, as org.json's getInt lets it.
    pub fn as_i64_lenient(&self) -> Option<i64> {
        match self {
            Json::Int(n) => Some(*n),
            Json::Float(f) if f.is_finite() => Some(f.trunc() as i64),
            Json::Str(s) => {
                let t = s.trim();
                t.parse::<i64>().ok().or_else(|| {
                    t.parse::<f64>()
                        .ok()
                        .filter(|f| f.is_finite())
                        .map(|f| f.trunc() as i64)
                })
            }
            _ => None,
        }
    }

    /// A number only, cut to a whole number: what `raw instanceof Number` then intValue()
    /// read in the old code.
    pub fn as_number_i64(&self) -> Option<i64> {
        match self {
            Json::Int(n) => Some(*n),
            Json::Float(f) if f.is_finite() => Some(f.trunc() as i64),
            _ => None,
        }
    }

    /// A boolean, or a string "true" or "false" in any case, as org.json's getBoolean.
    pub fn as_bool_lenient(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            Json::Str(s) if s.eq_ignore_ascii_case("true") => Some(true),
            Json::Str(s) if s.eq_ignore_ascii_case("false") => Some(false),
            _ => None,
        }
    }

    /// The document on one line with no spaces, keys in their order.
    pub fn to_compact(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, None, 0);
        out
    }

    /// The document indented by this many spaces per level.
    pub fn to_pretty(&self, indent: usize) -> String {
        let mut out = String::new();
        self.write(&mut out, Some(indent), 0);
        out
    }

    /// A form for comparing two documents whatever their key order and spacing: keys sorted,
    /// and every leaf tagged with its kind, so 1 and "1" and 1.0 stay apart.
    pub fn canonical(&self) -> String {
        let mut out = String::new();
        self.write_canonical(&mut out);
        out
    }

    fn write_canonical(&self, out: &mut String) {
        match self {
            Json::Object(entries) => {
                let mut sorted: Vec<&(String, Json)> = entries.iter().collect();
                sorted.sort_by(|a, b| a.0.cmp(&b.0));
                out.push('{');
                for (k, v) in sorted {
                    out.push_str(k);
                    out.push(':');
                    v.write_canonical(out);
                    out.push(',');
                }
                out.push('}');
            }
            Json::Array(items) => {
                out.push('[');
                for v in items {
                    v.write_canonical(out);
                    out.push(',');
                }
                out.push(']');
            }
            Json::Null => out.push_str("Null"),
            Json::Bool(b) => {
                let _ = write!(out, "Boolean {b}");
            }
            Json::Int(n) => {
                let _ = write!(out, "Integer {n}");
            }
            Json::Float(f) => {
                let _ = write!(out, "Double {f}");
            }
            Json::Str(s) => {
                let _ = write!(out, "String {s}");
            }
        }
    }

    fn write(&self, out: &mut String, indent: Option<usize>, depth: usize) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Int(n) => {
                let _ = write!(out, "{n}");
            }
            Json::Float(f) => write_float(out, *f),
            Json::Str(s) => write_string(out, s),
            Json::Array(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    newline(out, indent, depth + 1);
                    v.write(out, indent, depth + 1);
                }
                newline(out, indent, depth);
                out.push(']');
            }
            Json::Object(entries) => {
                if entries.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push('{');
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    newline(out, indent, depth + 1);
                    write_string(out, k);
                    out.push(':');
                    if indent.is_some() {
                        out.push(' ');
                    }
                    v.write(out, indent, depth + 1);
                }
                newline(out, indent, depth);
                out.push('}');
            }
        }
    }
}

fn newline(out: &mut String, indent: Option<usize>, depth: usize) {
    if let Some(n) = indent {
        out.push('\n');
        for _ in 0..n * depth {
            out.push(' ');
        }
    }
}

fn write_float(out: &mut String, f: f64) {
    if !f.is_finite() {
        // JSON has no way to say these; null is what a reader can at least parse
        out.push_str("null");
    } else if f.fract() == 0.0 && f.abs() < 1e15 {
        let _ = write!(out, "{}", f as i64);
    } else {
        let _ = write!(out, "{f}");
    }
}

fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

impl From<bool> for Json {
    fn from(b: bool) -> Self {
        Json::Bool(b)
    }
}

impl From<i32> for Json {
    fn from(n: i32) -> Self {
        Json::Int(n as i64)
    }
}

impl From<i64> for Json {
    fn from(n: i64) -> Self {
        Json::Int(n)
    }
}

impl From<u32> for Json {
    fn from(n: u32) -> Self {
        Json::Int(n as i64)
    }
}

impl From<usize> for Json {
    fn from(n: usize) -> Self {
        Json::Int(n as i64)
    }
}

impl From<&str> for Json {
    fn from(s: &str) -> Self {
        Json::Str(s.to_owned())
    }
}

impl From<String> for Json {
    fn from(s: String) -> Self {
        Json::Str(s)
    }
}

impl<T: Into<Json>> From<Vec<T>> for Json {
    fn from(items: Vec<T>) -> Self {
        Json::Array(items.into_iter().map(Into::into).collect())
    }
}

/// Nesting deeper than this is refused rather than allowed to exhaust the stack: a real
/// document here is three levels deep.
const MAX_DEPTH: usize = 128;

struct Parser<'a> {
    s: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn error(&self, what: &str) -> JsonError {
        JsonError(format!("{what} at character {}", self.at))
    }

    fn skip_ws(&mut self) {
        while self.at < self.s.len() && matches!(self.s[self.at], b' ' | b'\t' | b'\n' | b'\r') {
            self.at += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.at).copied()
    }

    fn expect(&mut self, word: &str) -> Result<(), JsonError> {
        if self.s[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            Ok(())
        } else {
            Err(self.error("unexpected text"))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, JsonError> {
        if depth > MAX_DEPTH {
            return Err(self.error("nested too deeply"));
        }
        match self.peek() {
            None => Err(self.error("unexpected end of the document")),
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') => self.expect("true").map(|_| Json::Bool(true)),
            Some(b'f') => self.expect("false").map(|_| Json::Bool(false)),
            Some(b'n') => self.expect("null").map(|_| Json::Null),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.number(),
            Some(_) => Err(self.error("unexpected character")),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json, JsonError> {
        self.at += 1;
        let mut entries: Vec<(String, Json)> = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.at += 1;
            return Ok(Json::Object(entries));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(self.error("expected a key"));
            }
            let key = self.string()?;
            self.skip_ws();
            if self.peek() != Some(b':') {
                return Err(self.error("expected ':'"));
            }
            self.at += 1;
            self.skip_ws();
            let value = self.value(depth + 1)?;
            // A key given twice keeps its last value, as org.json does
            if let Some(entry) = entries.iter_mut().find(|(k, _)| *k == key) {
                entry.1 = value;
            } else {
                entries.push((key, value));
            }
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;
                    return Ok(Json::Object(entries));
                }
                _ => return Err(self.error("expected ',' or '}'")),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Json, JsonError> {
        self.at += 1;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.at += 1;
            return Ok(Json::Array(items));
        }
        loop {
            self.skip_ws();
            items.push(self.value(depth + 1)?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err(self.error("expected ',' or ']'")),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, JsonError> {
        if self.at + 4 > self.s.len() {
            return Err(self.error("short \\u escape"));
        }
        let text = std::str::from_utf8(&self.s[self.at..self.at + 4])
            .map_err(|_| self.error("bad \\u escape"))?;
        let n = u32::from_str_radix(text, 16).map_err(|_| self.error("bad \\u escape"))?;
        self.at += 4;
        Ok(n)
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.at += 1;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let Some(c) = self.peek() else {
                return Err(self.error("unterminated string"));
            };
            self.at += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let Some(e) = self.peek() else {
                        return Err(self.error("unterminated escape"));
                    };
                    self.at += 1;
                    let ch = match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let first = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&first)
                                && self.s[self.at..].starts_with(b"\\u")
                            {
                                self.at += 2;
                                let second = self.hex4()?;
                                if (0xDC00..0xE000).contains(&second) {
                                    0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
                                } else {
                                    0xFFFD
                                }
                            } else {
                                first
                            };
                            char::from_u32(code).unwrap_or('\u{FFFD}')
                        }
                        _ => return Err(self.error("unknown escape")),
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                c => out.push(c),
            }
        }
        String::from_utf8(out).map_err(|_| self.error("string is not UTF-8"))
    }

    fn number(&mut self) -> Result<Json, JsonError> {
        let start = self.at;
        let mut float = false;
        while let Some(c) = self.peek() {
            match c {
                b'0'..=b'9' | b'-' | b'+' => {}
                b'.' | b'e' | b'E' => float = true,
                _ => break,
            }
            self.at += 1;
        }
        let text =
            std::str::from_utf8(&self.s[start..self.at]).map_err(|_| self.error("bad number"))?;
        if !float && let Ok(n) = text.parse::<i64>() {
            return Ok(Json::Int(n));
        }
        text.parse::<f64>()
            .map(Json::Float)
            .map_err(|_| self.error("bad number"))
    }
}
