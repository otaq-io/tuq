//! Text files as lines, read and written the way awk does (every line written back ends in a newline).
//!
//! A line ending in `\r\n` (a Windows checkout with `core.autocrlf`) is read without its `\r`, so markers and
//! headings still match, and is written back with it; lines the commands add take the ending of the marker line they
//! follow.

use std::io;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub text: Vec<u8>,
    pub cr: bool,
}

impl Line {
    pub fn new(text: impl Into<Vec<u8>>, cr: bool) -> Line {
        Line {
            text: text.into(),
            cr,
        }
    }
}

/// Splits on `\n` as awk splits records: a last line without a newline is still a line; an empty file has none.
pub fn split(data: &[u8]) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut rest = data;
    while !rest.is_empty() {
        let (line, next) = match rest.iter().position(|&b| b == b'\n') {
            Some(i) => (&rest[..i], &rest[i + 1..]),
            None => (rest, &rest[rest.len()..]),
        };
        match line.strip_suffix(b"\r") {
            Some(t) => lines.push(Line::new(t, true)),
            None => lines.push(Line::new(line, false)),
        }
        rest = next;
    }
    lines
}

pub fn join(lines: &[Line]) -> Vec<u8> {
    let mut out = Vec::new();
    for l in lines {
        out.extend_from_slice(&l.text);
        out.extend_from_slice(if l.cr { b"\r\n" } else { b"\n" });
    }
    out
}

pub fn read(path: &Path) -> io::Result<Vec<Line>> {
    Ok(split(&std::fs::read(path)?))
}

/// Writes `path.tmp`, then moves it over `path`, as the scripts did.
pub fn write_replacing(path: &Path, data: &[u8]) -> io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)
}

/// True when some line is exactly `text` (`grep -qxF`).
pub fn has_line(lines: &[Line], text: &[u8]) -> bool {
    lines.iter().any(|l| l.text == text)
}

/// The lines strictly between the first `a` line and the `b` line after it (to the end when there is none).
pub fn between<'a>(lines: &'a [Line], a: &[u8], b: &[u8]) -> Vec<&'a Line> {
    let mut on = false;
    let mut out = Vec::new();
    for l in lines {
        if l.text == a {
            on = true;
            continue;
        }
        if l.text == b {
            break;
        }
        if on {
            out.push(l);
        }
    }
    out
}

/// Replaces the lines strictly between each `a` line and the next `b` line; `content` goes after the first `a` line
/// only (later blocks are emptied, as the scripts' awk did); every other line stays.
pub fn replace_between(lines: &[Line], a: &[u8], b: &[u8], content: &[Vec<u8>]) -> Vec<Line> {
    let mut out = Vec::new();
    let mut skip = false;
    let mut written = false;
    for l in lines {
        if l.text == a {
            out.push(l.clone());
            if !written {
                out.extend(content.iter().map(|c| Line::new(c.clone(), l.cr)));
                written = true;
            }
            skip = true;
            continue;
        }
        if l.text == b {
            skip = false;
        }
        if !skip {
            out.push(l.clone());
        }
    }
    out
}

pub fn contains(hay: &[u8], needle: &[u8]) -> bool {
    needle.is_empty() || hay.windows(needle.len()).any(|w| w == needle)
}

pub fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

/// awk's `trim`: spaces and tabs off both ends.
pub fn trim(s: &[u8]) -> &[u8] {
    let is_blank = |b: &u8| *b == b' ' || *b == b'\t';
    let start = s.iter().position(|b| !is_blank(b)).unwrap_or(s.len());
    let end = s
        .iter()
        .rposition(|b| !is_blank(b))
        .map_or(start, |i| i + 1);
    &s[start..end.max(start)]
}

/// `^[ \t]*$`
pub fn is_blank(s: &[u8]) -> bool {
    s.iter().all(|&b| b == b' ' || b == b'\t')
}

/// awk's `s + 0` for a string of digits (with leading text ignored as awk would give 0).
pub fn number(s: &[u8]) -> u64 {
    let digits = s.iter().take_while(|b| b.is_ascii_digit());
    digits.fold(0u64, |n, &d| {
        n.saturating_mul(10).saturating_add(u64::from(d - b'0'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_keeps_a_last_line_without_newline_and_reads_crlf() {
        let l = split(b"a\r\nb\n\nc");
        assert_eq!(
            l,
            vec![
                Line::new("a", true),
                Line::new("b", false),
                Line::new("", false),
                Line::new("c", false)
            ]
        );
        assert_eq!(join(&l), b"a\r\nb\n\nc\n");
        assert!(split(b"").is_empty());
    }

    #[test]
    fn trim_and_number() {
        assert_eq!(trim(b" \t x y \t"), b"x y");
        assert_eq!(trim(b"   "), b"");
        assert_eq!(number(b"007"), 7);
        assert_eq!(number(b"x"), 0);
    }
}
