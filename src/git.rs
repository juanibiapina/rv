use std::fmt;
use std::process::Command;

use crate::error::Error;

const EMPTY_TREE_HASH: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

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

#[derive(Debug, Clone, PartialEq)]
pub struct Commit {
    pub hash: String,
    pub subject: String,
    pub parent_hash: Option<String>,
}

impl Commit {
    /// Returns git arguments to diff this commit against its parent.
    pub fn diff_args(&self) -> Vec<String> {
        let parent = self
            .parent_hash
            .as_deref()
            .unwrap_or(EMPTY_TREE_HASH);
        vec![
            "diff".into(),
            parent.into(),
            self.hash.clone(),
            "--no-ext-diff".into(),
        ]
    }

    /// Returns the first 7 characters of the commit hash.
    pub fn short_hash(&self) -> &str {
        &self.hash[..7.min(self.hash.len())]
    }
}

/// Parse `git log` output with tab-separated fields: hash, parents, subject.
pub fn parse_log(output: &str) -> Vec<Commit> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let mut parts = line.splitn(3, '\t');
            let hash = parts.next()?.trim().to_string();
            let parents_str = parts.next()?.trim();
            let subject = parts.next()?.trim().to_string();
            let parent_hash = parents_str
                .split_whitespace()
                .next()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            Some(Commit {
                hash,
                subject,
                parent_hash,
            })
        })
        .collect()
}

/// Load recent commits from the repository.
pub fn load_commits(limit: usize) -> Result<Vec<Commit>, Error> {
    let output = Command::new("git")
        .args([
            "log",
            &format!("--max-count={}", limit),
            "--pretty=format:%H\t%P\t%s",
        ])
        .output()
        .map_err(|e| Error::Git(format!("failed to run git: {}", e)))?;

    // git log exits non-zero on empty repos — that's not an error for us
    if !output.status.success() {
        return Ok(Vec::new());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_log(&text))
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
    fn parse_log_multiple_commits() {
        let output = "abc1234\tdef5678\tfirst commit\nghi9012\tabc1234\tsecond commit\n";
        let commits = parse_log(output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].hash, "abc1234");
        assert_eq!(commits[0].parent_hash, Some("def5678".into()));
        assert_eq!(commits[0].subject, "first commit");
        assert_eq!(commits[1].hash, "ghi9012");
        assert_eq!(commits[1].parent_hash, Some("abc1234".into()));
        assert_eq!(commits[1].subject, "second commit");
    }

    #[test]
    fn parse_log_empty() {
        assert_eq!(parse_log(""), Vec::<Commit>::new());
    }

    #[test]
    fn parse_log_single_commit() {
        let output = "abc1234\tdef5678\tonly commit";
        let commits = parse_log(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].hash, "abc1234");
    }

    #[test]
    fn parse_log_initial_commit() {
        let output = "abc1234\t\tinitial commit\n";
        let commits = parse_log(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].parent_hash, None);
    }

    #[test]
    fn parse_log_no_trailing_newline() {
        let output = "abc1234\tdef5678\tsome commit";
        let commits = parse_log(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].subject, "some commit");
    }

    #[test]
    fn parse_log_merge_commit() {
        let output = "abc1234\tparent1 parent2\tmerge branch\n";
        let commits = parse_log(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].parent_hash, Some("parent1".into()));
    }

    #[test]
    fn commit_diff_args_normal() {
        let commit = Commit {
            hash: "abc1234".into(),
            subject: "test".into(),
            parent_hash: Some("def5678".into()),
        };
        assert_eq!(
            commit.diff_args(),
            vec!["diff", "def5678", "abc1234", "--no-ext-diff"]
        );
    }

    #[test]
    fn commit_diff_args_initial() {
        let commit = Commit {
            hash: "abc1234".into(),
            subject: "initial".into(),
            parent_hash: None,
        };
        let args = commit.diff_args();
        assert_eq!(args[0], "diff");
        assert_eq!(args[1], EMPTY_TREE_HASH);
        assert_eq!(args[2], "abc1234");
        assert_eq!(args[3], "--no-ext-diff");
    }

    #[test]
    fn file_status_display() {
        assert_eq!(format!("{}", FileStatus::Added), "A");
        assert_eq!(format!("{}", FileStatus::Modified), "M");
        assert_eq!(format!("{}", FileStatus::Deleted), "D");
    }
}
