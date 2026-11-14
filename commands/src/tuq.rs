//! `tuq`: the mechanical part of `tuq/`.
//!
//! ```text
//! rules    print the rules of the tuq/ folder (rules.md, built in); every tuq skill reads them first
//! template <decision|readme|tech-spec|plan|orchestrator>   print a template
//! init     create tuq/README.md (+ decisions/) from the template when missing; print the repo root
//! next     print the next free decision number (001, 002, ... 999, 1000)
//! index    rebuild the decision index in tuq/README.md from the decision files
//! clashes  print numbers used by more than one decision file; exit 1 if any
//! ```
//!
//! Run from anywhere inside the repository.

use std::ffi::OsString;
use std::path::PathBuf;

use crate::lines::{self, Line};
use crate::root::{self, Root};
use crate::{arg, embedded, err, out};

pub const INDEX_START: &str = "<!-- tuq:index:start -->";
pub const INDEX_END: &str = "<!-- tuq:index:end -->";

pub fn run(args: Vec<OsString>) -> i32 {
    let result = match arg(&args, 0).as_str() {
        "rules" => {
            out(embedded::RULES.as_bytes());
            Ok(0)
        }
        "template" => template(&arg(&args, 1)),
        "init" => init(),
        "next" => next(),
        "index" => index(),
        "clashes" => clashes(),
        _ => {
            err("usage: tuq rules|template <name>|init|next|index|clashes");
            Ok(2)
        }
    };
    result.unwrap_or_else(|e| {
        err(&format!("tuq: {e}"));
        1
    })
}

/// Prints `tuq: <message>` and gives exit status 2.
fn die(message: &str) -> std::io::Result<i32> {
    err(&format!("tuq: {message}"));
    Ok(2)
}

fn template(name: &str) -> std::io::Result<i32> {
    match embedded::template(name) {
        Some(t) => {
            out(t.as_bytes());
            Ok(0)
        }
        None => die("usage: tuq template decision|readme|tech-spec|plan|orchestrator"),
    }
}

fn init() -> std::io::Result<i32> {
    let r = root::find();
    if !r.in_git {
        err("tuq: not in a git repository; using the current directory");
    }
    std::fs::create_dir_all(r.path().join("tuq").join("decisions"))?;
    let readme = r.path().join("tuq").join("README.md");
    if !readme.exists() {
        std::fs::write(&readme, embedded::TEMPLATE_README)?;
        err("tuq: created tuq/README.md");
    }
    out(format!("{}\n", r.text).as_bytes());
    Ok(0)
}

/// A decision file: `tuq/decisions/NNN-<slug>.md`.
pub struct DecisionFile {
    pub number: u64,
    /// The number as the file name writes it (`007`).
    pub prefix: Vec<u8>,
    pub name: String,
    pub path: PathBuf,
}

/// The decision files, sorted by number (numeric, so 1000 follows 999), then by name. Only regular files directly in
/// `tuq/decisions/` named `[0-9][0-9][0-9]*-*.md` whose part before the first `-` is all digits.
pub fn decision_files(root: &Root) -> std::io::Result<Vec<DecisionFile>> {
    let dir = root.path().join("tuq").join("decisions");
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let b = name.as_bytes();
        if !matches_decision_glob(b) {
            continue;
        }
        let dash = b.iter().position(|&c| c == b'-').unwrap_or(b.len());
        let prefix = &b[..dash];
        if prefix.is_empty() || !prefix.iter().all(u8::is_ascii_digit) {
            continue;
        }
        files.push(DecisionFile {
            number: lines::number(prefix),
            prefix: prefix.to_vec(),
            path: dir.join(&name),
            name,
        });
    }
    files.sort_by(|a, b| {
        a.number
            .cmp(&b.number)
            .then_with(|| a.name.as_bytes().cmp(b.name.as_bytes()))
    });
    Ok(files)
}

/// `find -name '[0-9][0-9][0-9]*-*.md'`
fn matches_decision_glob(b: &[u8]) -> bool {
    if b.len() < 3 || !b[..3].iter().all(u8::is_ascii_digit) {
        return false;
    }
    let rest = &b[3..];
    match rest.strip_suffix(b".md") {
        Some(head) => head.contains(&b'-'),
        None => false,
    }
}

fn next() -> std::io::Result<i32> {
    let r = root::find();
    let max = decision_files(&r)?
        .iter()
        .map(|f| f.number)
        .max()
        .unwrap_or(0);
    out(format!("{:03}\n", max.saturating_add(1)).as_bytes());
    Ok(0)
}

fn clashes() -> std::io::Result<i32> {
    let r = root::find();
    let files = decision_files(&r)?;
    let mut report = String::new();
    let mut i = 0;
    while i < files.len() {
        let j = files[i..]
            .iter()
            .position(|f| f.number != files[i].number)
            .map_or(files.len(), |k| i + k);
        if j - i > 1 {
            report.push_str(&format!("{:03}\n", files[i].number));
            for f in &files[i..j] {
                report.push_str(&format!("  tuq/decisions/{}\n", f.name));
            }
        }
        i = j;
    }
    if report.is_empty() {
        return Ok(0);
    }
    out(report.as_bytes());
    Ok(1)
}

/// One row of the index: the number, the linked title, the status, the summary.
fn index_row(file: &DecisionFile) -> std::io::Result<Result<Vec<u8>, String>> {
    let mut title: Vec<u8> = Vec::new();
    let mut status: Vec<u8> = Vec::new();
    let mut summary: Vec<u8> = Vec::new();
    let mut in_summary = false;
    for line in lines::read(&file.path)? {
        let l = line.text.as_slice();
        if title.is_empty() {
            if let Some(rest) = after_title_prefix(l) {
                title = lines::trim(rest).to_vec();
            }
        }
        if status.is_empty() && (l.starts_with(b"- **Status:**") || l.starts_with(b"* **Status:**"))
        {
            status = lines::trim(&l[b"- **Status:**".len()..]).to_vec();
        }
        if l.starts_with(b"## ") {
            in_summary = l.starts_with(b"## Summary") && lines::is_blank(&l[b"## Summary".len()..]);
            continue;
        }
        if in_summary {
            if lines::is_blank(l) {
                if !summary.is_empty() {
                    in_summary = false;
                }
                continue;
            }
            if !summary.is_empty() {
                summary.push(b' ');
            }
            summary.extend_from_slice(lines::trim(l));
        }
    }
    let mut missing = String::new();
    if title.is_empty() {
        missing.push_str(" title line (# NNN · title)");
    }
    if status.is_empty() {
        missing.push_str(" **Status:** line");
    }
    if summary.is_empty() {
        missing.push_str(" ## Summary paragraph");
    }
    if !missing.is_empty() {
        return Ok(Err(format!(
            "tuq: tuq/decisions/{} lacks:{missing}",
            file.name
        )));
    }
    let mut row = file.prefix.clone();
    row.extend_from_slice(b"\t[");
    row.extend(escape_pipes(&title));
    row.extend_from_slice(format!("](decisions/{})\t", file.name).as_bytes());
    row.extend(escape_pipes(&status));
    row.push(b'\t');
    row.extend(escape_pipes(&summary));
    Ok(Ok(row))
}

/// The text after `# <digits> · ` at the start of a line.
fn after_title_prefix(l: &[u8]) -> Option<&[u8]> {
    let rest = l.strip_prefix(b"# ")?;
    let digits = rest.iter().take_while(|b| b.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    rest[digits..].strip_prefix(" · ".as_bytes())
}

fn escape_pipes(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for &b in s {
        if b == b'|' {
            out.push(b'\\');
        }
        out.push(b);
    }
    out
}

/// The display width the scripts used: characters, counted as the bytes that don't continue a UTF-8 sequence.
fn width(s: &[u8]) -> usize {
    s.iter().filter(|&&b| !(0x80..=0xBF).contains(&b)).count()
}

fn pad(s: &[u8], n: usize) -> Vec<u8> {
    let mut out = s.to_vec();
    out.resize(s.len() + n.saturating_sub(width(s)), b' ');
    out
}

/// The index table, columns padded to their display width (as prettier does).
fn table(rows: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let header: [&[u8]; 4] = [b"#", b"Decision", b"Status", b"Summary"];
    let mut widths: Vec<usize> = header.iter().map(|h| width(h).max(3)).collect();
    let cells: Vec<Vec<Vec<u8>>> = rows
        .iter()
        .map(|r| {
            let mut parts: Vec<Vec<u8>> = r.split(|&b| b == b'\t').map(<[u8]>::to_vec).collect();
            parts.resize(4.max(parts.len()), Vec::new());
            parts.truncate(4);
            parts
        })
        .collect();
    for row in &cells {
        for (c, cell) in row.iter().enumerate() {
            widths[c] = widths[c].max(width(cell));
        }
    }
    let line = |cells: &[&[u8]]| {
        let mut l = b"|".to_vec();
        for (c, cell) in cells.iter().enumerate() {
            l.push(b' ');
            l.extend(pad(cell, widths[c]));
            l.extend_from_slice(b" |");
        }
        l
    };
    let mut out = vec![line(&header)];
    let dashes: Vec<Vec<u8>> = widths.iter().map(|&w| vec![b'-'; w]).collect();
    out.push(line(&dashes.iter().map(Vec::as_slice).collect::<Vec<_>>()));
    for row in &cells {
        out.push(line(&row.iter().map(Vec::as_slice).collect::<Vec<_>>()));
    }
    if rows.is_empty() {
        out.push(b"| | _No decisions yet._ | | |".to_vec());
    }
    out
}

fn index() -> std::io::Result<i32> {
    let r = root::find();
    let readme = r.path().join("tuq").join("README.md");
    if !readme.is_file() {
        return die("no tuq/README.md (run: tuq init)");
    }
    let readme_lines = lines::read(&readme)?;
    for marker in [INDEX_START, INDEX_END] {
        if !lines::has_line(&readme_lines, marker.as_bytes()) {
            return die(&format!("tuq/README.md has no '{marker}' marker"));
        }
    }
    let mut rows = Vec::new();
    for f in decision_files(&r)? {
        match index_row(&f)? {
            Ok(row) => rows.push(row),
            Err(message) => {
                err(&message);
                return Ok(1);
            }
        }
    }
    let table = table(&rows);
    let mut new_lines: Vec<Line> = Vec::new();
    let mut skip = false;
    for l in &readme_lines {
        if l.text == INDEX_START.as_bytes() {
            new_lines.push(l.clone());
            new_lines.push(Line::new("", l.cr));
            new_lines.extend(table.iter().map(|t| Line::new(t.clone(), l.cr)));
            new_lines.push(Line::new("", l.cr));
            skip = true;
            continue;
        }
        if l.text == INDEX_END.as_bytes() {
            skip = false;
        }
        if !skip {
            new_lines.push(l.clone());
        }
    }
    lines::write_replacing(&readme, &lines::join(&new_lines))?;
    err(&format!("tuq: index rebuilt ({} decisions)", rows.len()));
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_glob() {
        assert!(matches_decision_glob(b"001-a.md"));
        assert!(matches_decision_glob(b"001-.md"));
        assert!(matches_decision_glob(b"1000-big.md"));
        assert!(!matches_decision_glob(b"01-a.md"));
        assert!(!matches_decision_glob(b"005.md"));
        assert!(!matches_decision_glob(b"006-a.txt"));
        assert!(!matches_decision_glob(b"007.md-"));
    }

    #[test]
    fn width_counts_characters() {
        assert_eq!(width("a·b".as_bytes()), 3);
        assert_eq!(width("ğüş".as_bytes()), 3);
    }
}
