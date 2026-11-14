//! The repository root: `git rev-parse --show-toplevel`, or the current directory outside a git repository.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct Root {
    /// As git prints it (forward slashes on Windows too), or the current directory.
    pub text: String,
    pub in_git: bool,
}

impl Root {
    pub fn path(&self) -> PathBuf {
        PathBuf::from(&self.text)
    }

    /// `<root>/<rel>`, as the scripts printed it.
    pub fn show(&self, rel: &str) -> String {
        format!("{}/{}", self.text, rel)
    }
}

pub fn find() -> Root {
    let git = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    match git {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout)
                .trim_end_matches(['\n', '\r'])
                .to_string();
            if !text.is_empty() {
                return Root { text, in_git: true };
            }
            Root {
                text: pwd(),
                in_git: false,
            }
        }
        Ok(_) => Root {
            text: pwd(),
            in_git: false,
        },
        // No git to run: the nearest folder holding `.git` is the root.
        Err(_) => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let mut dir: Option<&Path> = Some(&cwd);
            while let Some(d) = dir {
                if d.join(".git").exists() {
                    return Root {
                        text: d.to_string_lossy().into_owned(),
                        in_git: true,
                    };
                }
                dir = d.parent();
            }
            Root {
                text: pwd(),
                in_git: false,
            }
        }
    }
}

/// The current directory as the shell's `pwd` prints it: `$PWD` when it names this directory, else the real path.
fn pwd() -> String {
    let real = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if let Some(p) = std::env::var_os("PWD") {
        let p = PathBuf::from(p);
        if p.is_absolute() && same_dir(&p, &real) {
            return p.to_string_lossy().into_owned();
        }
    }
    real.to_string_lossy().into_owned()
}

#[cfg(unix)]
fn same_dir(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(x), Ok(y)) => x.dev() == y.dev() && x.ino() == y.ino(),
        _ => false,
    }
}

#[cfg(not(unix))]
fn same_dir(_: &Path, _: &Path) -> bool {
    false
}
