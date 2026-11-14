//! Files with Windows line endings (a checkout with `core.autocrlf`): the bash scripts didn't see the markers in
//! them; the commands read them as the same lines and write them back with their `\r\n`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tuq-crlf-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    dir
}

fn write(dir: &Path, rel: &str, lf_text: &str) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, lf_text.replace('\n', "\r\n")).unwrap();
}

fn run(dir: &Path, exe: &str, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(exe)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn index_reads_and_keeps_crlf() {
    let dir = repo("index");
    write(
        &dir,
        "tuq/README.md",
        "# t\n<!-- tuq:index:start -->\nold\n<!-- tuq:index:end -->\nafter\n",
    );
    write(
        &dir,
        "tuq/decisions/001-a.md",
        "# 001 · First\n\n- **Status:** Accepted\n\n## Summary\n\nOne line.\n\n## Why\n",
    );
    let (status, _, stderr) = run(&dir, env!("CARGO_BIN_EXE_tuq"), &["index"]);
    assert_eq!(
        (status, stderr.as_str()),
        (0, "tuq: index rebuilt (1 decisions)\n")
    );
    let readme = fs::read_to_string(dir.join("tuq/README.md")).unwrap();
    let want = "# t\n<!-- tuq:index:start -->\n\n\
                | #   | Decision                    | Status   | Summary   |\n\
                | --- | --------------------------- | -------- | --------- |\n\
                | 001 | [First](decisions/001-a.md) | Accepted | One line. |\n\n<!-- tuq:index:end -->\nafter\n";
    assert_eq!(readme, want.replace('\n', "\r\n"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn plan_check_and_order_read_crlf() {
    let dir = repo("plan");
    write(
        &dir,
        "tuq/plans/demo.md",
        "<!-- tuq:plan:start -->\nSee tuq/prompts/demo-orchestrator.md.\n\n## Pieces\n\n1. A — waits for nothing\n   Check: a\n2. B — waits for 1\n   Check: b\n\n<!-- tuq:plan:end -->\n",
    );
    write(
        &dir,
        "tuq/prompts/demo-orchestrator.md",
        "tuq/plans/demo.md\n<!-- tuq:order:start -->\n<!-- tuq:order:end -->\n### Piece 2\n",
    );
    let plan = env!("CARGO_BIN_EXE_tuq-plan");
    let (status, stdout, _) = run(&dir, plan, &["check", "demo"]);
    assert_eq!((status, stdout.as_str()), (1, "tuq/prompts/demo-orchestrator.md:2: the run order is stale; run: tuq-plan order demo\n"));
    assert_eq!(run(&dir, plan, &["order", "demo"]).0, 0);
    assert_eq!(
        run(&dir, plan, &["check", "demo"]),
        (0, String::new(), String::new())
    );
    let prompt = fs::read_to_string(dir.join("tuq/prompts/demo-orchestrator.md")).unwrap();
    let want = "tuq/plans/demo.md\n<!-- tuq:order:start -->\n\nStart a piece as soon as every piece it waits for is merged:\n\n\
                - First: 1 A\n- After 1: 2 B\n\n<!-- tuq:order:end -->\n### Piece 2\n";
    assert_eq!(prompt, want.replace('\n', "\r\n"));
    let _ = fs::remove_dir_all(&dir);
}
