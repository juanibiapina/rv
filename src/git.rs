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

/// Returns diff args for the working tree (unstaged changes).
pub fn worktree_diff_args() -> Vec<String> {
    vec!["diff".into(), "--no-ext-diff".into()]
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

/// Get the raw unified diff for a single file.
pub fn file_diff(diff_args: &[String], file_path: &str) -> Result<String, Error> {
    let mut args: Vec<&str> = diff_args.iter().map(|s| s.as_str()).collect();
    args.push("--");
    args.push(file_path);

    let output = Command::new("git")
        .args(&args)
        .output()
        .map_err(|e| Error::Git(format!("failed to run git diff: {}", e)))?;

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
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
