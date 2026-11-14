//! `tuq-plan`: the mechanical part of `tuq:milestone-plan`.
//!
//! ```text
//! new   <feature>  create tuq/plans/<feature>.md and tuq/prompts/<feature>-orchestrator.md from the templates when
//!                  missing (existing files are left untouched); print both paths
//! check <feature>  validate the Pieces shape, the links and the run order; exit 1 naming file and line
//! order <feature>  rewrite the run order block in the prompt from the plan's Pieces
//! ```
//!
//! Run from anywhere inside the repository. The plan file stands alone: nothing here reads or writes an Otaq ticket
//! (tuq decision 093), so the scripts' `body` and `pull` are gone.

use std::ffi::OsString;
use std::path::PathBuf;

use crate::lines::{self, Line};
use crate::root::{self, Root};
use crate::{arg, embedded, err, out, today};

pub const PLAN_START: &str = "<!-- tuq:plan:start -->";
pub const PLAN_END: &str = "<!-- tuq:plan:end -->";
pub const ORDER_START: &str = "<!-- tuq:order:start -->";
pub const ORDER_END: &str = "<!-- tuq:order:end -->";

const EM_WAITS: &str = " — waits for ";

/// Ends the command: the exit status, after printing `tuq-plan: <message>` when it is a `die`.
enum Exit {
    Code(i32),
    Die(String),
}

impl From<std::io::Error> for Exit {
    fn from(e: std::io::Error) -> Exit {
        err(&format!("tuq-plan: {e}"));
        Exit::Code(1)
    }
}

type Done = Result<(), Exit>;

pub fn run(args: Vec<OsString>) -> i32 {
    let feature = arg(&args, 1);
    let result = match arg(&args, 0).as_str() {
        "new" => new(&feature),
        "check" => check(&feature),
        "order" => order(&feature),
        _ => {
            err("usage: tuq-plan new|check|order <feature>");
            Err(Exit::Code(2))
        }
    };
    match result {
        Ok(()) => 0,
        Err(Exit::Code(c)) => c,
        Err(Exit::Die(m)) => {
            err(&format!("tuq-plan: {m}"));
            2
        }
    }
}

fn die<T>(message: impl Into<String>) -> Result<T, Exit> {
    Err(Exit::Die(message.into()))
}

/// The feature name: kebab-case words, as in `tuq/tech-specs/<feature>.md`.
fn feature_arg(f: &str, command: &str) -> Result<String, Exit> {
    if f.is_empty() {
        return die(format!(
            "missing <feature> (usage: tuq-plan {command} <feature>)"
        ));
    }
    let kebab = f.split('-').all(|w| {
        !w.is_empty()
            && w.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    });
    if !kebab {
        return die(format!(
            "feature '{f}' is not kebab-case (e.g. reading-list-sharing)"
        ));
    }
    Ok(f.to_string())
}

fn plan_rel(f: &str) -> String {
    format!("tuq/plans/{f}.md")
}

fn prompt_rel(f: &str) -> String {
    format!("tuq/prompts/{f}-orchestrator.md")
}

/// A template with `<feature>` and the date filled in, every line ending in a newline.
pub fn from_template(template: &str, feature: &str, today: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for l in lines::split(template.as_bytes()) {
        let text = String::from_utf8_lossy(&l.text)
            .replace("<feature>", feature)
            .replace("YYYY-MM-DD", today);
        out.extend(lines::join(&[Line::new(text, l.cr)]));
    }
    out
}

fn new(feature: &str) -> Done {
    let feature = feature_arg(feature, "new")?;
    let r = root::find();
    if !r.in_git {
        err("tuq-plan: not in a git repository; using the current directory");
    }
    let today = today::today();
    for (rel, template) in [
        (plan_rel(&feature), embedded::TEMPLATE_PLAN),
        (prompt_rel(&feature), embedded::TEMPLATE_ORCHESTRATOR),
    ] {
        let dest = r.path().join(&rel);
        if dest.exists() {
            err(&format!("tuq-plan: {rel} exists; left untouched"));
        } else {
            if let Some(dir) = dest.parent() {
                std::fs::create_dir_all(dir)?;
            }
            lines::write_replacing(&dest, &from_template(template, &feature, &today))?;
            err(&format!("tuq-plan: created {rel}"));
        }
        out(format!("{}\n", r.show(&rel)).as_bytes());
    }
    Ok(())
}

/// What every command but `new` works on: the feature, both paths; the plan must exist.
struct Loaded {
    feature: String,
    plan_rel: String,
    prompt_rel: String,
    plan: PathBuf,
    prompt: PathBuf,
}

fn load(feature: &str, command: &str) -> Result<Loaded, Exit> {
    let feature = feature_arg(feature, command)?;
    let r: Root = root::find();
    let (plan_rel, prompt_rel) = (plan_rel(&feature), prompt_rel(&feature));
    let (plan, prompt) = (r.path().join(&plan_rel), r.path().join(&prompt_rel));
    if !plan.is_file() {
        return die(format!(
            "{plan_rel} not found (create it with: tuq-plan new {feature})"
        ));
    }
    Ok(Loaded {
        feature,
        plan_rel,
        prompt_rel,
        plan,
        prompt,
    })
}

// ---------------------------------------------------------------------------------------------------------------
// Parsing the plan. One pass over the plan file gives one record per line, tab-separated:
//   P <n> <title> <waits>     a piece: its number, its title, the numbers it waits for (space-separated, or empty)
//   E <line> <message>        a shape error (line 0: the file as a whole)
// The records are split back on tabs where they are read, as the scripts did.

#[derive(Default)]
struct Parser {
    records: Vec<Vec<u8>>,
    start: usize,
    stop: usize,
    in_markers: bool,
    pieces_at: usize,
    names_prompt: bool,
    np: u64,
    cur: u64,
    cur_line: usize,
    details: u64,
    has_check: bool,
}

impl Parser {
    fn err(&mut self, line: usize, message: &str) {
        self.records
            .push(format!("E\t{line}\t{message}").into_bytes());
    }

    fn piece(&mut self, n: u64, title: &[u8], waits: &str) {
        let mut r = format!("P\t{n}\t").into_bytes();
        r.extend_from_slice(title);
        r.push(b'\t');
        r.extend_from_slice(waits.as_bytes());
        self.records.push(r);
    }

    fn close_piece(&mut self) {
        if self.cur == 0 {
            return;
        }
        if self.details == 0 {
            let m = format!("piece {} has no indented details under its title", self.cur);
            self.err(self.cur_line, &m);
        } else if !self.has_check {
            let m = format!("piece {} has no \"Check:\" line in its details", self.cur);
            self.err(self.cur_line, &m);
        }
        self.cur = 0;
    }

    fn start_piece(&mut self, nr: usize, line: &[u8]) {
        self.close_piece();
        self.np += 1;
        let n = lines::number(line);
        if n != self.np {
            let m = format!(
                "piece numbered {n}, expected {} (pieces are numbered 1, 2, 3 in order)",
                self.np
            );
            self.err(nr, &m);
        }
        self.cur = n;
        self.cur_line = nr;
        self.details = 0;
        self.has_check = false;
        // The title line after `N.` and its spaces.
        let digits = line.iter().take_while(|b| b.is_ascii_digit()).count();
        let after_dot = &line[digits + 1..];
        let rest = &after_dot[after_dot.iter().take_while(|&&b| b == b' ').count()..];
        let Some(at) = lines::find(rest, EM_WAITS.as_bytes()) else {
            let wrong_dash = [" - waits for ", " -- waits for ", " – waits for "]
                .iter()
                .any(|d| lines::contains(rest, d.as_bytes()));
            if wrong_dash {
                self.err(
                    nr,
                    "use an em dash (—) before \"waits for\", not a hyphen or en dash",
                );
            } else {
                let m = format!("title line must read \"{n}. Title — waits for nothing\" or \"— waits for 1, 2\"");
                self.err(nr, &m);
            }
            self.piece(n, rest, "");
            return;
        };
        let title = &rest[..at];
        let waits = String::from_utf8_lossy(&rest[at + EM_WAITS.len()..]).into_owned();
        let well_formed = after_dot.len() >= 2 && after_dot[0] == b' ' && after_dot[1] != b' ';
        if !well_formed || title.is_empty() {
            self.err(
                nr,
                &format!("title line must read \"{n}. Title — waits for ...\""),
            );
        }
        if waits == "nothing" {
            self.piece(n, title, "");
            return;
        }
        let numbers_list = waits
            .split(", ")
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
        if !numbers_list {
            self.err(nr, &format!("\"waits for\" must be \"nothing\" or piece numbers like \"1, 2\", got \"{waits}\""));
            self.piece(n, title, "");
            return;
        }
        let mut w: Vec<String> = Vec::new();
        for part in waits.split(", ") {
            let p = lines::number(part.as_bytes());
            if p < 1 || p >= n {
                self.err(
                    nr,
                    &format!("piece {n} waits for {part}: a piece waits only for pieces before it"),
                );
            } else {
                w.push(p.to_string());
            }
        }
        self.piece(n, title, &w.join(" "));
    }

    fn line(&mut self, nr: usize, l: &[u8], prompt: &[u8]) {
        if marker_line(l, PLAN_START) {
            self.start = nr;
            self.in_markers = true;
            return;
        }
        if marker_line(l, PLAN_END) {
            self.stop = nr;
            if self.in_markers {
                self.close_piece();
            }
            self.in_markers = false;
            return;
        }
        if !self.in_markers {
            return;
        }
        if self.pieces_at == 0 {
            if l.starts_with(b"## Pieces") && lines::is_blank(&l[b"## Pieces".len()..]) {
                self.pieces_at = nr;
            } else if lines::contains(l, prompt) {
                self.names_prompt = true;
            }
            return;
        }
        if l.starts_with(b"#") {
            self.err(
                nr,
                "a heading after \"## Pieces\": the Pieces list must come last in the ticket Plan",
            );
            return;
        }
        if lines::is_blank(l) {
            return;
        }
        let digits = l.iter().take_while(|b| b.is_ascii_digit()).count();
        if digits > 0 && l.get(digits) == Some(&b'.') {
            self.start_piece(nr, l);
            return;
        }
        if l.starts_with(b" ") || l.starts_with(b"\t") {
            if self.cur == 0 {
                self.err(nr, "indented text with no piece title above it");
                return;
            }
            self.details += 1;
            let indent = l.iter().take_while(|&&b| b == b' ' || b == b'\t').count();
            if l[indent..].starts_with(b"Check:") {
                self.has_check = true;
            }
            return;
        }
        self.err(
            nr,
            "neither a piece title (\"N. Title — waits for ...\") nor details indented under one",
        );
    }

    fn end(&mut self, prompt: &str) {
        if self.start == 0 {
            self.err(0, &format!("missing {PLAN_START}"));
        }
        if self.stop == 0 {
            self.err(0, &format!("missing {PLAN_END}"));
        }
        if self.start != 0 && self.stop != 0 && self.stop < self.start {
            self.err(
                self.stop,
                &format!("{PLAN_END} comes before the start marker"),
            );
        }
        if self.start != 0 && self.pieces_at == 0 {
            self.err(
                self.start,
                "no \"## Pieces\" heading between the plan markers",
            );
        }
        if self.start != 0 && self.pieces_at != 0 && self.np == 0 {
            self.err(self.pieces_at, "no pieces under \"## Pieces\"");
        }
        if self.start != 0 && !prompt.is_empty() && !self.names_prompt {
            let m = format!(
                "the why (before \"## Pieces\") must name the orchestrator prompt {prompt}"
            );
            self.err(self.start, &m);
        }
    }
}

/// The marker alone on its line (trailing spaces and tabs allowed).
fn marker_line(l: &[u8], marker: &str) -> bool {
    l.strip_prefix(marker.as_bytes())
        .is_some_and(lines::is_blank)
}

/// The records of the plan file at `plan`; `prompt` is the prompt's path the why must name.
fn parse(plan_lines: &[Line], prompt: &str) -> Vec<Vec<u8>> {
    let mut p = Parser::default();
    for (i, l) in plan_lines.iter().enumerate() {
        p.line(i + 1, &l.text, prompt.as_bytes());
    }
    p.end(prompt);
    p.records
}

fn fields(record: &[u8]) -> Vec<&[u8]> {
    record.split(|&b| b == b'\t').collect()
}

fn is_error(record: &[u8]) -> bool {
    record.starts_with(b"E")
}

/// The E records as `file:line: message` (`file: message` for line 0).
fn error_lines(records: &[Vec<u8>], plan_rel: &str) -> Vec<Vec<u8>> {
    records
        .iter()
        .filter_map(|r| {
            let f = fields(r);
            if f[0] != b"E" {
                return None;
            }
            let line = f.get(1).copied().unwrap_or(b"");
            let mut out = plan_rel.as_bytes().to_vec();
            if lines::number(line) != 0 {
                out.push(b':');
                out.extend_from_slice(line);
            }
            out.extend_from_slice(b": ");
            out.extend_from_slice(f.get(2).copied().unwrap_or(b""));
            Some(out)
        })
        .collect()
}

/// The run order block, from the P records: a piece's step is one more than the latest step of what it waits for;
/// each step is one line naming what it comes after (the waits of its pieces).
fn order_block(records: &[Vec<u8>]) -> Vec<Vec<u8>> {
    use std::collections::{BTreeSet, HashMap};
    let mut title: HashMap<u64, Vec<u8>> = HashMap::new();
    let mut waits: HashMap<u64, Vec<u8>> = HashMap::new();
    let mut max = 0;
    for r in records {
        let f = fields(r);
        if f[0] != b"P" {
            continue;
        }
        let n = lines::number(f.get(1).copied().unwrap_or(b""));
        title.insert(n, f.get(2).copied().unwrap_or(b"").to_vec());
        waits.insert(n, f.get(3).copied().unwrap_or(b"").to_vec());
        max = max.max(n);
    }
    let words = |n: u64| -> Vec<Vec<u8>> {
        let w = waits.get(&n).cloned().unwrap_or_default();
        w.split(|b| b.is_ascii_whitespace())
            .filter(|t| !t.is_empty())
            .map(<[u8]>::to_vec)
            .collect()
    };
    // Steps by the waits' text, as awk's array subscripts.
    let mut step: HashMap<Vec<u8>, u64> = HashMap::new();
    let mut last = 0;
    for n in 1..=max {
        let mut s = 1;
        for w in words(n) {
            s = s.max(step.get(&w).copied().unwrap_or(0) + 1);
        }
        step.insert(n.to_string().into_bytes(), s);
        last = last.max(s);
    }
    let mut out: Vec<Vec<u8>> = vec![
        Vec::new(),
        b"Start a piece as soon as every piece it waits for is merged:".to_vec(),
        Vec::new(),
    ];
    for s in 1..=last {
        let mut list: Vec<u8> = Vec::new();
        let mut count = 0;
        let mut after: BTreeSet<u64> = BTreeSet::new();
        for n in 1..=max {
            if step.get(n.to_string().as_bytes()).copied().unwrap_or(0) != s {
                continue;
            }
            if count > 0 {
                list.extend_from_slice(b", ");
            }
            list.extend_from_slice(format!("{n} ").as_bytes());
            list.extend(title.get(&n).cloned().unwrap_or_default());
            count += 1;
            for w in words(n) {
                after.insert(lines::number(&w));
            }
        }
        if count == 0 {
            continue;
        }
        let mut line = if s == 1 {
            b"- First: ".to_vec()
        } else {
            let a: Vec<String> = after
                .iter()
                .filter(|&&n| n >= 1 && n <= max)
                .map(u64::to_string)
                .collect();
            format!("- After {}: ", a.join(", ")).into_bytes()
        };
        line.extend(list);
        if count > 1 {
            line.extend_from_slice(b" (side by side)");
        }
        out.push(line);
    }
    out.push(Vec::new());
    out
}

fn order(feature: &str) -> Done {
    let l = load(feature, "order")?;
    if !l.prompt.is_file() {
        return die(format!(
            "{} not found (create it with: tuq-plan new {})",
            l.prompt_rel, l.feature
        ));
    }
    let prompt_lines = lines::read(&l.prompt)?;
    if !lines::has_line(&prompt_lines, ORDER_START.as_bytes())
        || !lines::has_line(&prompt_lines, ORDER_END.as_bytes())
    {
        return die(format!(
            "{} has no run order markers ({ORDER_START} ... {ORDER_END})",
            l.prompt_rel
        ));
    }
    let records = parse(&lines::read(&l.plan)?, &l.prompt_rel);
    if records.iter().any(|r| is_error(r)) {
        for e in error_lines(&records, &l.plan_rel) {
            err(&String::from_utf8_lossy(&e));
        }
        return die("fix the plan first; the run order was not written");
    }
    let new_lines = lines::replace_between(
        &prompt_lines,
        ORDER_START.as_bytes(),
        ORDER_END.as_bytes(),
        &order_block(&records),
    );
    lines::write_replacing(&l.prompt, &lines::join(&new_lines))?;
    Ok(())
}

/// Lines joined with newlines, without the newlines at the end (what `$(...)` keeps).
fn joined(lines: &[&[u8]]) -> Vec<u8> {
    let mut s = lines.join(&b'\n');
    while s.last() == Some(&b'\n') {
        s.pop();
    }
    s
}

fn check(feature: &str) -> Done {
    let l = load(feature, "check")?;
    let records = parse(&lines::read(&l.plan)?, &l.prompt_rel);
    let mut errors = error_lines(&records, &l.plan_rel);
    let mut add = |s: String| errors.push(s.into_bytes());
    if !l.prompt.is_file() {
        add(format!(
            "{}: missing (create it with: tuq-plan new {})",
            l.prompt_rel, l.feature
        ));
    } else {
        let raw = std::fs::read(&l.prompt)?;
        let prompt_lines = lines::split(&raw);
        if !lines::contains(&raw, l.plan_rel.as_bytes()) {
            add(format!(
                "{}: doesn't link to the plan {}",
                l.prompt_rel, l.plan_rel
            ));
        }
        let has_markers = lines::has_line(&prompt_lines, ORDER_START.as_bytes())
            && lines::has_line(&prompt_lines, ORDER_END.as_bytes());
        if has_markers {
            if !records.iter().any(|r| is_error(r)) {
                let want_lines = order_block(&records);
                let want = joined(&want_lines.iter().map(Vec::as_slice).collect::<Vec<_>>());
                let have_lines =
                    lines::between(&prompt_lines, ORDER_START.as_bytes(), ORDER_END.as_bytes());
                let have = joined(
                    &have_lines
                        .iter()
                        .map(|l| l.text.as_slice())
                        .collect::<Vec<_>>(),
                );
                if have != want {
                    let at = prompt_lines
                        .iter()
                        .position(|l| l.text == ORDER_START.as_bytes())
                        .map_or(0, |i| i + 1);
                    add(format!(
                        "{}:{at}: the run order is stale; run: tuq-plan order {}",
                        l.prompt_rel, l.feature
                    ));
                }
            }
        } else {
            add(format!(
                "{}: missing run order markers ({ORDER_START} ... {ORDER_END})",
                l.prompt_rel
            ));
        }
        let max = records
            .iter()
            .filter(|r| r.starts_with(b"P\t"))
            .map(|r| lines::number(fields(r).get(1).copied().unwrap_or(b"")))
            .max()
            .unwrap_or(0);
        for (i, pl) in prompt_lines.iter().enumerate() {
            let Some(rest) = pl.text.strip_prefix(b"### Piece ") else {
                continue;
            };
            let digits = rest.iter().take_while(|b| b.is_ascii_digit()).count();
            if digits == 0 || !lines::is_blank(&rest[digits..]) {
                continue;
            }
            let n = lines::number(rest);
            if !(1..=max).contains(&n) {
                add(format!(
                    "{}:{}: note for piece {n}, but the plan has pieces 1 to {max}",
                    l.prompt_rel,
                    i + 1
                ));
            }
        }
    }
    if errors.is_empty() {
        return Ok(());
    }
    let mut text = Vec::new();
    for e in errors {
        text.extend(e);
        text.push(b'\n');
    }
    out(&text);
    Err(Exit::Code(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn records(plan: &str) -> Vec<String> {
        parse(
            &lines::split(plan.as_bytes()),
            "tuq/prompts/x-orchestrator.md",
        )
        .into_iter()
        .map(|r| String::from_utf8(r).unwrap())
        .collect()
    }

    #[test]
    fn parses_pieces_and_waits() {
        let plan = "<!-- tuq:plan:start -->\nSee tuq/prompts/x-orchestrator.md.\n## Pieces\n1. A — waits for nothing\n   Check: a\n2. B — waits for 1\n   Check: b\n<!-- tuq:plan:end -->\n";
        assert_eq!(records(plan), vec!["P\t1\tA\t", "P\t2\tB\t1"]);
    }

    #[test]
    fn order_steps_side_by_side() {
        let recs: Vec<Vec<u8>> = [
            "P\t1\tA\t",
            "P\t2\tB\t",
            "P\t3\tC\t1",
            "P\t4\tD\t1 2",
            "P\t5\tE\t3 4",
            "P\t6\tF\t2",
        ]
        .iter()
        .map(|s| s.as_bytes().to_vec())
        .collect();
        let block: Vec<String> = order_block(&recs)
            .into_iter()
            .map(|l| String::from_utf8(l).unwrap())
            .collect();
        assert_eq!(
            block,
            vec![
                "",
                "Start a piece as soon as every piece it waits for is merged:",
                "",
                "- First: 1 A, 2 B (side by side)",
                "- After 1, 2: 3 C, 4 D, 6 F (side by side)",
                "- After 3, 4: 5 E",
                ""
            ]
        );
    }
}
