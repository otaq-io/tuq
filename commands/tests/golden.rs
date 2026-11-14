//! Golden tests: each case in `tests/cases/<name>/` runs one command on a scratch repository and compares what it
//! prints, its exit status and the repository afterwards with `expected/`.
//!
//! The expected output was recorded from the bash scripts these programs replace (`plugins/tuq/bin/tuq` and
//! `tuq-plan`, deleted once these programs matched them; commit a6ee2d7 still has them). To record again from a
//! copy of them:
//!
//! ```sh
//! TUQ_RECORD_BASH=/path/to/plugins/tuq/bin cargo test --test golden
//! ```
//!
//! A case:
//! - `command`: the first line is the command (`tuq index`, `tuq-plan check demo`); later lines are options:
//!   `nogit` (the scratch folder is not a git repository), `cwd <dir>` (run from that folder of the repository).
//! - `repo/`: the repository before the command (none: an empty repository).
//! - `expected/stdout`, `stderr`, `status`, `tree.txt` (every folder and file afterwards) and `repo/` (the files).
//!   The scratch folder's path reads `@ROOT@`. A file holding only `@@FILE <path under plugins/tuq/>@@` stands for
//!   that file as it is now (`feature=<name>` fills in a plan template as `tuq-plan new` does, `@TODAY@` the date),
//!   so the rules' and templates' text can change without breaking these tests.
//! - `deliberate`: the expected output differs from the bash scripts on purpose (the file says why); recording
//!   leaves the case alone.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn plugin_dir() -> PathBuf {
    manifest().join("..").join("plugins").join("tuq")
}

struct Case {
    name: String,
    dir: PathBuf,
    program: String,
    args: Vec<String>,
    git: bool,
    cwd: Option<String>,
}

fn load_cases() -> Vec<Case> {
    let mut cases = Vec::new();
    let root = manifest().join("tests").join("cases");
    let mut names: Vec<_> = fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    names.sort();
    for name in names {
        let dir = root.join(&name);
        let text = fs::read_to_string(dir.join("command")).unwrap();
        let mut lines = text.lines();
        let mut words = lines.next().unwrap().split_whitespace().map(str::to_string);
        let program = words.next().unwrap();
        let args = words.collect();
        let (mut git, mut cwd) = (true, None);
        for opt in lines {
            match opt.split_once(' ') {
                _ if opt == "nogit" => git = false,
                Some(("cwd", d)) => cwd = Some(d.to_string()),
                _ if opt.trim().is_empty() => {}
                _ => panic!("{}: unknown option {opt}", name.to_string_lossy()),
            }
        }
        cases.push(Case {
            name: name.to_string_lossy().into_owned(),
            dir,
            program,
            args,
            git,
            cwd,
        });
    }
    cases
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    if !from.is_dir() {
        return;
    }
    for e in fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let dest = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_tree(&e.path(), &dest);
        } else {
            fs::copy(e.path(), &dest).unwrap();
        }
    }
}

/// Every folder (`name/`) and file under `dir`, but `.git`, sorted, one per line.
fn tree(dir: &Path) -> Vec<String> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
        for e in fs::read_dir(dir).unwrap() {
            let e = e.unwrap();
            if e.file_name() == ".git" {
                continue;
            }
            let rel = e
                .path()
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if e.file_type().unwrap().is_dir() {
                out.push(format!("{rel}/"));
                walk(base, &e.path(), out);
            } else {
                out.push(rel);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tuq-golden-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The ways the scratch folder's path can appear in output, longest first.
fn root_spellings(dir: &Path, git: bool) -> Vec<String> {
    let mut s = vec![dir.to_string_lossy().into_owned()];
    if let Ok(c) = fs::canonicalize(dir) {
        let c = c.to_string_lossy().into_owned();
        s.push(c.strip_prefix(r"\\?\").unwrap_or(&c).to_string());
    }
    if git {
        let out = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(dir)
            .output()
            .unwrap();
        s.push(String::from_utf8_lossy(&out.stdout).trim_end().to_string());
    }
    for v in s.clone() {
        s.push(v.replace('\\', "/"));
    }
    s.sort_by_key(|v| std::cmp::Reverse(v.len()));
    s.dedup();
    s
}

fn normalize(bytes: &[u8], roots: &[String]) -> String {
    let mut text = String::from_utf8_lossy(bytes).into_owned();
    for r in roots {
        if !r.is_empty() {
            text = text.replace(r.as_str(), "@ROOT@");
        }
    }
    text
}

/// `(stub, text)` for each file under plugins/tuq/ an output may be, the plan templates filled in for `feature`.
fn known_files(feature: Option<&str>, today: &str) -> Vec<(String, Vec<u8>)> {
    let mut known = Vec::new();
    for rel in [
        "rules.md",
        "templates/decision.md",
        "templates/README.md",
        "templates/tech-spec.md",
        "templates/plan.md",
        "templates/orchestrator.md",
    ] {
        known.push((
            format!("@@FILE {rel}@@\n"),
            fs::read(plugin_dir().join(rel)).unwrap(),
        ));
        if let (Some(f), true) = (
            feature,
            rel == "templates/plan.md" || rel == "templates/orchestrator.md",
        ) {
            let t = fs::read_to_string(plugin_dir().join(rel)).unwrap();
            let filled = tuq_commands::plan::from_template(&t, f, today);
            known.push((format!("@@FILE {rel} feature={f}@@\n"), filled));
        }
    }
    known
}

fn resolve(expected: &[u8], known: &[(String, Vec<u8>)]) -> Vec<u8> {
    let text = String::from_utf8_lossy(expected);
    if text.starts_with("@@FILE ") {
        for (stub, content) in known {
            if *stub == text {
                return content.clone();
            }
        }
        panic!("unknown stub {text}");
    }
    expected.to_vec()
}

fn stub_for(actual: &[u8], known: &[(String, Vec<u8>)]) -> Vec<u8> {
    // The filled-in template first: a plain one could match only if it had no <feature> in it.
    for (stub, content) in known.iter().rev() {
        if content == actual {
            return stub.clone().into_bytes();
        }
    }
    actual.to_vec()
}

struct Run {
    stdout: String,
    stderr: String,
    status: i32,
    tree: Vec<String>,
    dir: PathBuf,
}

fn run_case(case: &Case, bash: Option<&Path>) -> Run {
    let dir = scratch(&case.name);
    copy_tree(&case.dir.join("repo"), &dir);
    if case.git {
        let ok = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&dir)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git init");
    }
    let cwd = match &case.cwd {
        Some(d) => dir.join(d),
        None => dir.clone(),
    };
    let mut cmd = match bash {
        Some(bin) => {
            let mut c = Command::new("bash");
            c.arg(bin.join(&case.program));
            c
        }
        None => Command::new(match case.program.as_str() {
            "tuq" => env!("CARGO_BIN_EXE_tuq"),
            "tuq-plan" => env!("CARGO_BIN_EXE_tuq-plan"),
            p => panic!("unknown program {p}"),
        }),
    };
    let out = cmd
        .args(&case.args)
        .current_dir(&cwd)
        .env_remove("PWD")
        .output()
        .unwrap();
    let roots = root_spellings(&dir, case.git);
    Run {
        stdout: normalize(&out.stdout, &roots),
        stderr: normalize(&out.stderr, &roots),
        status: out.status.code().unwrap_or(-1),
        tree: tree(&dir),
        dir,
    }
}

fn feature_of(case: &Case) -> Option<&str> {
    (case.program == "tuq-plan")
        .then(|| case.args.get(1).map(String::as_str))
        .flatten()
}

fn record(case: &Case, bin: &Path) {
    let today = tuq_commands::today::today();
    let known = known_files(feature_of(case), &today);
    let run = run_case(case, Some(bin));
    let exp = case.dir.join("expected");
    let _ = fs::remove_dir_all(&exp);
    fs::create_dir_all(exp.join("repo")).unwrap();
    fs::write(exp.join("stdout"), stub_for(run.stdout.as_bytes(), &known)).unwrap();
    fs::write(exp.join("stderr"), &run.stderr).unwrap();
    fs::write(exp.join("status"), format!("{}\n", run.status)).unwrap();
    fs::write(
        exp.join("tree.txt"),
        run.tree
            .iter()
            .map(|t| format!("{t}\n"))
            .collect::<String>(),
    )
    .unwrap();
    for t in run.tree.iter().filter(|t| !t.ends_with('/')) {
        let dest = exp.join("repo").join(t);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::write(dest, stub_for(&fs::read(run.dir.join(t)).unwrap(), &known)).unwrap();
    }
    let _ = fs::remove_dir_all(&run.dir);
}

fn compare(case: &Case) -> Vec<String> {
    let mut diffs = Vec::new();
    let today = tuq_commands::today::today();
    let known = known_files(feature_of(case), &today);
    let run = run_case(case, None);
    let exp = case.dir.join("expected");
    let read = |p: &str| {
        fs::read(exp.join(p)).unwrap_or_else(|_| panic!("{}: no expected/{p}", case.name))
    };
    let mut check = |what: &str, want: Vec<u8>, have: &[u8]| {
        if want != have {
            diffs.push(format!(
                "{}: {what} differs\n--- expected\n{}\n--- actual\n{}",
                case.name,
                String::from_utf8_lossy(&want),
                String::from_utf8_lossy(have)
            ));
        }
    };
    check(
        "stdout",
        resolve(&read("stdout"), &known),
        run.stdout.as_bytes(),
    );
    check("stderr", read("stderr"), run.stderr.as_bytes());
    check(
        "status",
        read("status"),
        format!("{}\n", run.status).as_bytes(),
    );
    let tree_text: String = run.tree.iter().map(|t| format!("{t}\n")).collect();
    check("tree", read("tree.txt"), tree_text.as_bytes());
    for t in run.tree.iter().filter(|t| !t.ends_with('/')) {
        let want = fs::read(exp.join("repo").join(t))
            .map(|w| resolve(&w, &known))
            .unwrap_or_default();
        check(
            &format!("file {t}"),
            want,
            &fs::read(run.dir.join(t)).unwrap(),
        );
    }
    let _ = fs::remove_dir_all(&run.dir);
    diffs
}

#[test]
fn golden_cases() {
    let cases = load_cases();
    assert!(!cases.is_empty());
    if let Some(bin) = std::env::var_os("TUQ_RECORD_BASH") {
        for case in cases.iter().filter(|c| !c.dir.join("deliberate").exists()) {
            record(case, Path::new(&bin));
        }
        return;
    }
    let diffs: Vec<String> = cases.iter().flat_map(compare).collect();
    assert!(
        diffs.is_empty(),
        "{} of {} cases differ:\n\n{}",
        diffs.len(),
        cases.len(),
        diffs.join("\n\n")
    );
}
