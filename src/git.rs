use std::collections::HashMap;
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

/// Get raw unified diffs for all changed files in a single git call.
///
/// Returns a map from file path to raw diff output.
pub fn all_file_diffs(diff_args: &[String]) -> Result<HashMap<String, String>, Error> {
    let args: Vec<&str> = diff_args.iter().map(|s| s.as_str()).collect();

    let output = Command::new("git")
        .args(&args)
        .output()
        .map_err(|e| Error::Git(format!("failed to run git diff: {}", e)))?;

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(split_combined_diff(&text))
}

/// Split combined multi-file diff output into per-file chunks.
///
/// Each chunk is keyed by the file path extracted from the diff preamble.
pub fn split_combined_diff(output: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();

    // Find byte positions where "diff --git " starts a line
    let mut positions = Vec::new();
    if output.starts_with("diff --git ") {
        positions.push(0);
    }
    for (i, _) in output.match_indices("\ndiff --git ") {
        positions.push(i + 1); // skip the \n
    }

    for (idx, &start) in positions.iter().enumerate() {
        let end = positions.get(idx + 1).copied().unwrap_or(output.len());
        let chunk = &output[start..end];
        if let Some(path) = extract_diff_path(chunk) {
            result.insert(path.to_string(), chunk.to_string());
        }
    }

    result
}

/// Extract the file path from a single-file diff chunk.
///
/// Looks for `+++ b/<path>` first (works for added and modified files).
/// Falls back to `--- a/<path>` for deleted files where `+++ /dev/null`.
fn extract_diff_path(chunk: &str) -> Option<&str> {
    let mut minus_path = None;

    for line in chunk.lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            return Some(path);
        }
        if let Some(path) = line.strip_prefix("--- a/") {
            minus_path = Some(path);
        }
        // Stop scanning after the preamble
        if line.starts_with("@@ ") {
            break;
        }
    }

    // Deleted files have +++ /dev/null, so use the --- a/ path
    minus_path
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

    // --- split_combined_diff tests ---

    #[test]
    fn split_empty_input() {
        let result = split_combined_diff("");
        assert!(result.is_empty());
    }

    #[test]
    fn split_single_file() {
        let input = "\
diff --git a/src/main.rs b/src/main.rs
index abc..def 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,1 +1,1 @@
-old
+new
";
        let result = split_combined_diff(input);
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("src/main.rs"));
        assert!(result["src/main.rs"].contains("-old"));
        assert!(result["src/main.rs"].contains("+new"));
    }

    #[test]
    fn split_multiple_files() {
        let input = "\
diff --git a/src/main.rs b/src/main.rs
index abc..def 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,1 +1,1 @@
-old
+new
diff --git a/src/lib.rs b/src/lib.rs
index 123..456 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,1 +1,1 @@
-old lib
+new lib
";
        let result = split_combined_diff(input);
        assert_eq!(result.len(), 2);
        assert!(result.contains_key("src/main.rs"));
        assert!(result.contains_key("src/lib.rs"));
        assert!(result["src/lib.rs"].contains("-old lib"));
    }

    #[test]
    fn split_added_file() {
        let input = "\
diff --git a/new.rs b/new.rs
new file mode 100644
index 0000000..abc1234
--- /dev/null
+++ b/new.rs
@@ -0,0 +1,2 @@
+line one
+line two
";
        let result = split_combined_diff(input);
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("new.rs"));
    }

    #[test]
    fn split_deleted_file() {
        let input = "\
diff --git a/old.rs b/old.rs
deleted file mode 100644
index abc1234..0000000
--- a/old.rs
+++ /dev/null
@@ -1,2 +0,0 @@
-line one
-line two
";
        let result = split_combined_diff(input);
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("old.rs"));
    }

    #[test]
    fn split_no_hunks() {
        // Mode-only change with no diff content
        let input = "\
diff --git a/script.sh b/script.sh
old mode 100644
new mode 100755
";
        let result = split_combined_diff(input);
        // No --- or +++ lines, so no path extracted
        assert!(result.is_empty());
    }
}
