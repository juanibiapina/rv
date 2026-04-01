use std::fmt;
use std::process::Command;

use crate::error::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
}

impl fmt::Display for FileStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileStatus::Added => write!(f, "A"),
            FileStatus::Modified => write!(f, "M"),
            FileStatus::Deleted => write!(f, "D"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileEntry {
    pub path: String,
    pub status: FileStatus,
}

/// Parse `git diff --name-status` output into FileEntry values.
pub fn parse_name_status(output: &str) -> Vec<FileEntry> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let mut parts = line.splitn(2, '\t');
            let status_str = parts.next()?.trim();
            let path = parts.next()?.trim().to_string();
            let status = match status_str {
                "A" => FileStatus::Added,
                "M" => FileStatus::Modified,
                "D" => FileStatus::Deleted,
                _ => return None, // Skip rename, copy, etc.
            };
            Some(FileEntry { path, status })
        })
        .collect()
}

/// Check if the current directory is inside a git repository.
pub fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if the working tree has uncommitted changes (staged or unstaged).
pub fn is_dirty() -> bool {
    Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false)
}

/// Auto-detect the default remote branch (e.g., "refs/remotes/origin/main").
pub fn detect_base_branch() -> Result<String, Error> {
    let output = Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
        .output()
        .map_err(|e| Error::Git(format!("failed to run git: {}", e)))?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !branch.is_empty() {
            return Ok(branch);
        }
    }

    Err(Error::Git(
        "could not detect base branch. Is origin/HEAD set?".to_string(),
    ))
}

/// Compute the merge base between the given ref and HEAD.
pub fn merge_base(base_ref: &str) -> Result<String, Error> {
    let output = Command::new("git")
        .args(["merge-base", base_ref, "HEAD"])
        .output()
        .map_err(|e| Error::Git(format!("failed to run git: {}", e)))?;

    if output.status.success() {
        let base = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !base.is_empty() {
            return Ok(base);
        }
    }

    Err(Error::Git(format!(
        "could not find merge base with {}",
        base_ref
    )))
}

/// Compute diff arguments based on workspace state.
/// Returns (diff_args, description) or an error.
pub fn compute_diff_args() -> Result<(Vec<String>, String), Error> {
    if is_dirty() {
        return Ok((
            vec!["diff".into(), "HEAD".into(), "--no-ext-diff".into()],
            "uncommitted changes".into(),
        ));
    }

    let base_ref = detect_base_branch()?;
    let base = merge_base(&base_ref)?;
    Ok((
        vec![
            "diff".into(),
            format!("{}...HEAD", base),
            "--no-ext-diff".into(),
        ],
        format!("changes vs {}", base_ref),
    ))
}

/// Get the list of changed files using the given diff args.
pub fn changed_files(diff_args: &[String]) -> Result<Vec<FileEntry>, Error> {
    let mut args: Vec<&str> = diff_args.iter().map(|s| s.as_str()).collect();
    args.push("--name-status");

    let output = Command::new("git")
        .args(&args)
        .output()
        .map_err(|e| Error::Git(format!("failed to run git: {}", e)))?;

    if !output.status.success() {
        return Err(Error::Git("git diff --name-status failed".to_string()));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_name_status(&text))
}

/// Get the raw diff for a single file, rendered through delta.
pub fn file_diff_with_delta(diff_args: &[String], file_path: &str) -> Result<Vec<u8>, Error> {
    let mut git_args: Vec<String> = diff_args.to_vec();
    git_args.push("--".into());
    git_args.push(file_path.into());

    let escaped_args: Vec<String> = git_args
        .iter()
        .map(|a| format!("'{}'", a.replace('\'', "'\\''")))
        .collect();
    let cmd = format!(
        "git {} | delta --no-gitconfig --paging=never",
        escaped_args.join(" ")
    );

    let output = Command::new("bash")
        .args(["-c", &cmd])
        .output()
        .map_err(|e| Error::Git(format!("failed to run delta: {}", e)))?;

    Ok(output.stdout)
}

/// Check if delta is available.
pub fn has_delta() -> bool {
    Command::new("delta")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_name_status_mixed() {
        let output = "A\tsrc/main.rs\nM\tsrc/lib.rs\nD\told_file.txt\n";
        let files = parse_name_status(output);
        assert_eq!(
            files,
            vec![
                FileEntry {
                    path: "src/main.rs".into(),
                    status: FileStatus::Added,
                },
                FileEntry {
                    path: "src/lib.rs".into(),
                    status: FileStatus::Modified,
                },
                FileEntry {
                    path: "old_file.txt".into(),
                    status: FileStatus::Deleted,
                },
            ]
        );
    }

    #[test]
    fn parse_name_status_empty() {
        assert_eq!(parse_name_status(""), Vec::<FileEntry>::new());
    }

    #[test]
    fn parse_name_status_blank_lines() {
        let output = "\n  \nA\tfoo.rs\n\n";
        let files = parse_name_status(output);
        assert_eq!(
            files,
            vec![FileEntry {
                path: "foo.rs".into(),
                status: FileStatus::Added,
            }]
        );
    }

    #[test]
    fn parse_name_status_skips_rename() {
        // Renames show as R100, which we skip
        let output = "R100\told.rs\tnew.rs\nM\tkeep.rs\n";
        let files = parse_name_status(output);
        // R100 line has tab-separated old\tnew, splitn(2, '\t') gives ("R100", "old.rs\tnew.rs")
        // status "R100" doesn't match A/M/D, so it's skipped
        assert_eq!(
            files,
            vec![FileEntry {
                path: "keep.rs".into(),
                status: FileStatus::Modified,
            }]
        );
    }

    #[test]
    fn parse_name_status_single_added() {
        let output = "A\tnew_file.txt\n";
        let files = parse_name_status(output);
        assert_eq!(
            files,
            vec![FileEntry {
                path: "new_file.txt".into(),
                status: FileStatus::Added,
            }]
        );
    }

    #[test]
    fn parse_name_status_path_with_spaces() {
        let output = "M\tpath with spaces/file.rs\n";
        let files = parse_name_status(output);
        assert_eq!(
            files,
            vec![FileEntry {
                path: "path with spaces/file.rs".into(),
                status: FileStatus::Modified,
            }]
        );
    }

    #[test]
    fn parse_name_status_no_trailing_newline() {
        let output = "A\tfoo.rs";
        let files = parse_name_status(output);
        assert_eq!(
            files,
            vec![FileEntry {
                path: "foo.rs".into(),
                status: FileStatus::Added,
            }]
        );
    }

    #[test]
    fn file_status_display() {
        assert_eq!(format!("{}", FileStatus::Added), "A");
        assert_eq!(format!("{}", FileStatus::Modified), "M");
        assert_eq!(format!("{}", FileStatus::Deleted), "D");
    }
}
