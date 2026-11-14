//! The `tuq` and `tuq-plan` commands: the mechanical part of the tuq skills (`tuq/` at the repository root).
//!
//! They replace the bash scripts that were in `plugins/tuq/bin/`, with the same behaviour and messages, so they run
//! on macOS, Windows and Linux with nothing else installed (tuq decisions 064, 077). `rules.md` and the templates are
//! built into the programs from `plugins/tuq/`, so a program always matches the skills it ships with.

pub mod embedded;
pub mod lines;
pub mod plan;
pub mod root;
pub mod today;
pub mod tuq;

use std::io::Write;

/// The tuq release these programs were built for: `scripts/release` sets `TUQ_RELEASE` (`2026.11.14`) when it builds
/// them; a plain `cargo build` says `dev`. `tuq --version` and `tuq-plan --version` print it.
pub const VERSION: &str = match option_env!("TUQ_RELEASE") {
    Some(v) => v,
    None => "dev",
};

/// Writes to stdout; a closed pipe (`tuq rules | head`) is not an error.
pub fn out(bytes: &[u8]) {
    let mut s = std::io::stdout().lock();
    let _ = s.write_all(bytes);
    let _ = s.flush();
}

/// Writes one line to stderr.
pub fn err(line: &str) {
    let mut s = std::io::stderr().lock();
    let _ = s.write_all(line.as_bytes());
    let _ = s.write_all(b"\n");
}

/// The first argument as a string ("" when missing), as bash's `${1:-}`.
pub(crate) fn arg(args: &[std::ffi::OsString], i: usize) -> String {
    args.get(i)
        .map(|a| a.to_string_lossy().into_owned())
        .unwrap_or_default()
}
