use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRepoSummary {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub head_oid: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub staged_count: u32,
    pub unstaged_count: u32,
    pub untracked_count: u32,
    pub has_working_tree_changes: bool,
}

fn empty_summary() -> GitRepoSummary {
    GitRepoSummary {
        is_repo: false,
        branch: None,
        head_oid: None,
        upstream: None,
        ahead: 0,
        behind: 0,
        staged_count: 0,
        unstaged_count: 0,
        untracked_count: 0,
        has_working_tree_changes: false,
    }
}

/// Parses `git status --porcelain=v2 --branch` output with zero allocations per ignored line.
pub fn parse_porcelain_v2(output: &str) -> GitRepoSummary {
    let mut summary = GitRepoSummary {
        is_repo: true,
        ..empty_summary()
    };

    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("# branch.oid ") {
            if rest != "(initial)" {
                summary.head_oid = Some(rest.trim().to_string());
            }
        } else if let Some(rest) = line.strip_prefix("# branch.head ") {
            if rest != "(detached)" {
                summary.branch = Some(rest.trim().to_string());
            }
        } else if let Some(rest) = line.strip_prefix("# branch.upstream ") {
            summary.upstream = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            for part in rest.split_whitespace() {
                if let Some(a) = part.strip_prefix('+') {
                    summary.ahead = a.parse().unwrap_or(0);
                } else if let Some(b) = part.strip_prefix('-') {
                    summary.behind = b.parse().unwrap_or(0);
                }
            }
        } else if line.starts_with("1 ") || line.starts_with("2 ") {
            if let Some(xy) = line.split_whitespace().nth(1) {
                let mut chars = xy.chars();
                let x = chars.next().unwrap_or('.');
                let y = chars.next().unwrap_or('.');
                if x != '.' {
                    summary.staged_count += 1;
                }
                if y != '.' {
                    summary.unstaged_count += 1;
                }
            }
        } else if line.starts_with("u ") {
            summary.staged_count += 1;
            summary.unstaged_count += 1;
        } else if line.starts_with("? ") {
            summary.untracked_count += 1;
        }
    }

    summary.has_working_tree_changes =
        summary.staged_count > 0 || summary.unstaged_count > 0 || summary.untracked_count > 0;
    summary
}

/// Fast local-only git working-tree summary using `git status --porcelain=v2 --branch`.
pub fn git_repo_summary(cwd: &Path) -> GitRepoSummary {
    let mut cmd = Command::new("git");
    cmd.args([
        "--no-optional-locks",
        "status",
        "--porcelain=v2",
        "--branch",
        "--untracked-files=normal",
    ])
    .current_dir(cwd)
    .env("GIT_TERMINAL_PROMPT", "0");

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let Ok(output) = cmd.output() else {
        return empty_summary();
    };
    if !output.status.success() {
        return empty_summary();
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_porcelain_v2(&stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_porcelain_v2_branch_and_counts() {
        let sample = "\
# branch.oid 0123456789abcdef
# branch.head main
# branch.upstream origin/main
# branch.ab +3 -1
1 M. N... 100644 100644 100644 aaa bbb src/lib.rs
1 .M N... 100644 100644 100644 ccc ddd src/main.rs
1 MM N... 100644 100644 100644 eee fff Cargo.toml
? untracked.txt
";
        let summary = parse_porcelain_v2(sample);
        assert!(summary.is_repo);
        assert_eq!(summary.branch.as_deref(), Some("main"));
        assert_eq!(summary.upstream.as_deref(), Some("origin/main"));
        assert_eq!(summary.ahead, 3);
        assert_eq!(summary.behind, 1);
        assert_eq!(summary.staged_count, 2);
        assert_eq!(summary.unstaged_count, 2);
        assert_eq!(summary.untracked_count, 1);
        assert!(summary.has_working_tree_changes);
    }
}
