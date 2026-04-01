/// Types and parser for unified diff output, transformed into side-by-side rows.

/// The kind of a line in the unified diff.
#[derive(Debug, Clone, PartialEq)]
enum DiffLineKind {
    Context,
    Added,
    Deleted,
}

/// A parsed line from a unified diff hunk.
#[derive(Debug, Clone, PartialEq)]
struct DiffLine {
    kind: DiffLineKind,
    content: String,
    old_lineno: Option<usize>,
    new_lineno: Option<usize>,
}

/// The kind of content on one side of a side-by-side row.
#[derive(Debug, Clone, PartialEq)]
pub enum RowKind {
    Context,
    Added,
    Deleted,
}

/// Content for one side (left or right) of a side-by-side row.
#[derive(Debug, Clone, PartialEq)]
pub struct SideContent {
    pub lineno: usize,
    pub content: String,
    pub kind: RowKind,
}

/// A single row in the side-by-side view.
#[derive(Debug, Clone, PartialEq)]
pub enum SideBySideRow {
    /// A row with left and/or right content.
    Line {
        left: Option<SideContent>,
        right: Option<SideContent>,
    },
    /// Visual separator between hunks.
    HunkSeparator,
}

/// The complete side-by-side diff for a file.
#[derive(Debug, Clone, PartialEq)]
pub struct SideBySideDiff {
    pub rows: Vec<SideBySideRow>,
}

/// Parse a hunk header like `@@ -old_start,old_count +new_start,new_count @@`
/// Returns (old_start, new_start) or None if the line isn't a valid hunk header.
fn parse_hunk_header(line: &str) -> Option<(usize, usize)> {
    let line = line.strip_prefix("@@ ")?;
    let rest = line.split(" @@").next()?;
    let mut parts = rest.split_whitespace();

    let old_part = parts.next()?.strip_prefix('-')?;
    let old_start: usize = old_part.split(',').next()?.parse().ok()?;

    let new_part = parts.next()?.strip_prefix('+')?;
    let new_start: usize = new_part.split(',').next()?.parse().ok()?;

    Some((old_start, new_start))
}

/// Parse unified diff output into a list of DiffLines with line numbers.
fn parse_lines(input: &str) -> Vec<(DiffLineKind, DiffLine)> {
    let mut result = Vec::new();
    let mut old_lineno: usize = 0;
    let mut new_lineno: usize = 0;
    let mut in_hunk = false;

    for line in input.lines() {
        if let Some((old_start, new_start)) = parse_hunk_header(line) {
            old_lineno = old_start;
            new_lineno = new_start;
            in_hunk = true;
            // Push a sentinel so we know where hunks start
            result.push((
                DiffLineKind::Context,
                DiffLine {
                    kind: DiffLineKind::Context,
                    content: String::new(),
                    old_lineno: None,
                    new_lineno: None,
                },
            ));
            continue;
        }

        if !in_hunk {
            continue;
        }

        if line.starts_with("\\ ") {
            // "\ No newline at end of file" - skip
            continue;
        }

        if let Some(content) = line.strip_prefix(' ') {
            result.push((
                DiffLineKind::Context,
                DiffLine {
                    kind: DiffLineKind::Context,
                    content: content.to_string(),
                    old_lineno: Some(old_lineno),
                    new_lineno: Some(new_lineno),
                },
            ));
            old_lineno += 1;
            new_lineno += 1;
        } else if let Some(content) = line.strip_prefix('-') {
            result.push((
                DiffLineKind::Deleted,
                DiffLine {
                    kind: DiffLineKind::Deleted,
                    content: content.to_string(),
                    old_lineno: Some(old_lineno),
                    new_lineno: None,
                },
            ));
            old_lineno += 1;
        } else if let Some(content) = line.strip_prefix('+') {
            result.push((
                DiffLineKind::Added,
                DiffLine {
                    kind: DiffLineKind::Added,
                    content: content.to_string(),
                    old_lineno: None,
                    new_lineno: Some(new_lineno),
                },
            ));
            new_lineno += 1;
        }
        // Other lines (shouldn't appear inside a hunk) are skipped
    }

    result
}

/// Transform parsed diff lines into side-by-side rows.
///
/// Groups consecutive delete/add runs and pairs them as modifications.
/// Context lines appear on both sides. Unpaired deletes or adds get a
/// blank on the opposite side.
fn lines_to_rows(parsed: Vec<(DiffLineKind, DiffLine)>) -> Vec<SideBySideRow> {
    let mut rows = Vec::new();
    let mut i = 0;
    let mut first_hunk = true;

    while i < parsed.len() {
        let (ref kind, ref line) = parsed[i];

        // Hunk separator sentinel (empty line with no line numbers)
        if line.old_lineno.is_none() && line.new_lineno.is_none() {
            if !first_hunk {
                rows.push(SideBySideRow::HunkSeparator);
            }
            first_hunk = false;
            i += 1;
            continue;
        }

        match kind {
            DiffLineKind::Context => {
                rows.push(SideBySideRow::Line {
                    left: Some(SideContent {
                        lineno: line.old_lineno.unwrap(),
                        content: line.content.clone(),
                        kind: RowKind::Context,
                    }),
                    right: Some(SideContent {
                        lineno: line.new_lineno.unwrap(),
                        content: line.content.clone(),
                        kind: RowKind::Context,
                    }),
                });
                i += 1;
            }
            DiffLineKind::Deleted => {
                // Collect consecutive deletes
                let del_start = i;
                while i < parsed.len() && parsed[i].0 == DiffLineKind::Deleted {
                    i += 1;
                }
                let deletes = &parsed[del_start..i];

                // Collect consecutive adds that follow
                let add_start = i;
                while i < parsed.len() && parsed[i].0 == DiffLineKind::Added {
                    i += 1;
                }
                let adds = &parsed[add_start..i];

                let max_len = deletes.len().max(adds.len());
                for j in 0..max_len {
                    let left = deletes.get(j).map(|(_, dl)| SideContent {
                        lineno: dl.old_lineno.unwrap(),
                        content: dl.content.clone(),
                        kind: RowKind::Deleted,
                    });
                    let right = adds.get(j).map(|(_, al)| SideContent {
                        lineno: al.new_lineno.unwrap(),
                        content: al.content.clone(),
                        kind: RowKind::Added,
                    });
                    rows.push(SideBySideRow::Line { left, right });
                }
            }
            DiffLineKind::Added => {
                // Adds without preceding deletes
                rows.push(SideBySideRow::Line {
                    left: None,
                    right: Some(SideContent {
                        lineno: line.new_lineno.unwrap(),
                        content: line.content.clone(),
                        kind: RowKind::Added,
                    }),
                });
                i += 1;
            }
        }
    }

    rows
}

/// Parse raw `git diff` output into a side-by-side diff.
pub fn parse_side_by_side(input: &str) -> SideBySideDiff {
    let parsed = parse_lines(input);
    let rows = lines_to_rows(parsed);
    SideBySideDiff { rows }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_hunk_header tests ---

    #[test]
    fn hunk_header_basic() {
        assert_eq!(parse_hunk_header("@@ -1,3 +1,4 @@"), Some((1, 1)));
    }

    #[test]
    fn hunk_header_single_line() {
        assert_eq!(parse_hunk_header("@@ -1 +1 @@"), Some((1, 1)));
    }

    #[test]
    fn hunk_header_with_context_label() {
        assert_eq!(
            parse_hunk_header("@@ -10,5 +12,7 @@ fn main()"),
            Some((10, 12))
        );
    }

    #[test]
    fn hunk_header_not_a_header() {
        assert_eq!(parse_hunk_header("not a header"), None);
        assert_eq!(parse_hunk_header("--- a/file"), None);
        assert_eq!(parse_hunk_header("+++ b/file"), None);
    }

    // --- parse_side_by_side integration tests ---

    #[test]
    fn empty_input() {
        let diff = parse_side_by_side("");
        assert!(diff.rows.is_empty());
    }

    #[test]
    fn preamble_only_no_hunks() {
        let input = "diff --git a/foo b/foo\nindex abc..def 100644\n--- a/foo\n+++ b/foo\n";
        let diff = parse_side_by_side(input);
        assert!(diff.rows.is_empty());
    }

    #[test]
    fn context_only() {
        let input = "\
diff --git a/f b/f
--- a/f
+++ b/f
@@ -1,3 +1,3 @@
 line one
 line two
 line three
";
        let diff = parse_side_by_side(input);
        assert_eq!(diff.rows.len(), 3);
        for (i, row) in diff.rows.iter().enumerate() {
            match row {
                SideBySideRow::Line { left, right } => {
                    let l = left.as_ref().unwrap();
                    let r = right.as_ref().unwrap();
                    assert_eq!(l.lineno, i + 1);
                    assert_eq!(r.lineno, i + 1);
                    assert_eq!(l.kind, RowKind::Context);
                    assert_eq!(r.kind, RowKind::Context);
                    assert_eq!(l.content, r.content);
                }
                _ => panic!("expected Line row"),
            }
        }
    }

    #[test]
    fn add_only() {
        let input = "\
@@ -0,0 +1,2 @@
+new line one
+new line two
";
        let diff = parse_side_by_side(input);
        assert_eq!(diff.rows.len(), 2);
        match &diff.rows[0] {
            SideBySideRow::Line { left, right } => {
                assert!(left.is_none());
                let r = right.as_ref().unwrap();
                assert_eq!(r.lineno, 1);
                assert_eq!(r.content, "new line one");
                assert_eq!(r.kind, RowKind::Added);
            }
            _ => panic!("expected Line row"),
        }
        match &diff.rows[1] {
            SideBySideRow::Line { left, right } => {
                assert!(left.is_none());
                let r = right.as_ref().unwrap();
                assert_eq!(r.lineno, 2);
                assert_eq!(r.content, "new line two");
            }
            _ => panic!("expected Line row"),
        }
    }

    #[test]
    fn delete_only() {
        let input = "\
@@ -1,2 +0,0 @@
-old line one
-old line two
";
        let diff = parse_side_by_side(input);
        assert_eq!(diff.rows.len(), 2);
        match &diff.rows[0] {
            SideBySideRow::Line { left, right } => {
                assert!(right.is_none());
                let l = left.as_ref().unwrap();
                assert_eq!(l.lineno, 1);
                assert_eq!(l.content, "old line one");
                assert_eq!(l.kind, RowKind::Deleted);
            }
            _ => panic!("expected Line row"),
        }
    }

    #[test]
    fn modification_paired() {
        let input = "\
@@ -1,2 +1,2 @@
-old one
-old two
+new one
+new two
";
        let diff = parse_side_by_side(input);
        assert_eq!(diff.rows.len(), 2);
        match &diff.rows[0] {
            SideBySideRow::Line { left, right } => {
                let l = left.as_ref().unwrap();
                let r = right.as_ref().unwrap();
                assert_eq!(l.content, "old one");
                assert_eq!(l.kind, RowKind::Deleted);
                assert_eq!(l.lineno, 1);
                assert_eq!(r.content, "new one");
                assert_eq!(r.kind, RowKind::Added);
                assert_eq!(r.lineno, 1);
            }
            _ => panic!("expected Line row"),
        }
        match &diff.rows[1] {
            SideBySideRow::Line { left, right } => {
                let l = left.as_ref().unwrap();
                let r = right.as_ref().unwrap();
                assert_eq!(l.content, "old two");
                assert_eq!(r.content, "new two");
            }
            _ => panic!("expected Line row"),
        }
    }

    #[test]
    fn modification_more_deletes_than_adds() {
        let input = "\
@@ -1,3 +1,1 @@
-old one
-old two
-old three
+new one
";
        let diff = parse_side_by_side(input);
        assert_eq!(diff.rows.len(), 3);
        // First row: paired
        match &diff.rows[0] {
            SideBySideRow::Line { left, right } => {
                assert!(left.is_some());
                assert!(right.is_some());
            }
            _ => panic!("expected Line row"),
        }
        // Rows 2-3: delete only (right is None)
        match &diff.rows[1] {
            SideBySideRow::Line { left, right } => {
                assert!(left.is_some());
                assert!(right.is_none());
            }
            _ => panic!("expected Line row"),
        }
        match &diff.rows[2] {
            SideBySideRow::Line { left, right } => {
                assert!(left.is_some());
                assert!(right.is_none());
            }
            _ => panic!("expected Line row"),
        }
    }

    #[test]
    fn modification_more_adds_than_deletes() {
        let input = "\
@@ -1,1 +1,3 @@
-old one
+new one
+new two
+new three
";
        let diff = parse_side_by_side(input);
        assert_eq!(diff.rows.len(), 3);
        // First row: paired
        match &diff.rows[0] {
            SideBySideRow::Line { left, right } => {
                assert!(left.is_some());
                assert!(right.is_some());
            }
            _ => panic!("expected Line row"),
        }
        // Rows 2-3: add only (left is None)
        match &diff.rows[1] {
            SideBySideRow::Line { left, right } => {
                assert!(left.is_none());
                assert!(right.is_some());
            }
            _ => panic!("expected Line row"),
        }
        match &diff.rows[2] {
            SideBySideRow::Line { left, right } => {
                assert!(left.is_none());
                assert!(right.is_some());
            }
            _ => panic!("expected Line row"),
        }
    }

    #[test]
    fn multiple_hunks_with_separator() {
        let input = "\
@@ -1,2 +1,2 @@
 context one
-old
+new
@@ -10,2 +10,2 @@
 context ten
-old ten
+new ten
";
        let diff = parse_side_by_side(input);
        // Hunk 1: context + modification = 2 rows
        // Separator: 1
        // Hunk 2: context + modification = 2 rows
        assert_eq!(diff.rows.len(), 5);
        assert_eq!(diff.rows[2], SideBySideRow::HunkSeparator);
    }

    #[test]
    fn no_newline_marker_skipped() {
        let input = "\
@@ -1,1 +1,1 @@
-old
\\ No newline at end of file
+new
\\ No newline at end of file
";
        let diff = parse_side_by_side(input);
        assert_eq!(diff.rows.len(), 1);
        match &diff.rows[0] {
            SideBySideRow::Line { left, right } => {
                let l = left.as_ref().unwrap();
                let r = right.as_ref().unwrap();
                assert_eq!(l.content, "old");
                assert_eq!(r.content, "new");
            }
            _ => panic!("expected Line row"),
        }
    }

    #[test]
    fn git_preamble_skipped() {
        let input = "\
diff --git a/src/main.rs b/src/main.rs
index 1234567..abcdefg 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,1 +1,1 @@
-old
+new
";
        let diff = parse_side_by_side(input);
        assert_eq!(diff.rows.len(), 1);
    }

    #[test]
    fn adds_without_preceding_deletes() {
        let input = "\
@@ -1,2 +1,4 @@
 existing
+added one
+added two
 also existing
";
        let diff = parse_side_by_side(input);
        assert_eq!(diff.rows.len(), 4);
        // Row 0: context
        match &diff.rows[0] {
            SideBySideRow::Line { left, right } => {
                assert!(left.is_some());
                assert!(right.is_some());
                assert_eq!(left.as_ref().unwrap().kind, RowKind::Context);
            }
            _ => panic!("expected Line row"),
        }
        // Row 1: add only
        match &diff.rows[1] {
            SideBySideRow::Line { left, right } => {
                assert!(left.is_none());
                assert!(right.is_some());
                assert_eq!(right.as_ref().unwrap().content, "added one");
            }
            _ => panic!("expected Line row"),
        }
        // Row 2: add only
        match &diff.rows[2] {
            SideBySideRow::Line { left, right } => {
                assert!(left.is_none());
                assert!(right.is_some());
                assert_eq!(right.as_ref().unwrap().content, "added two");
            }
            _ => panic!("expected Line row"),
        }
        // Row 3: context
        match &diff.rows[3] {
            SideBySideRow::Line { left, right } => {
                assert!(left.is_some());
                assert!(right.is_some());
                assert_eq!(left.as_ref().unwrap().kind, RowKind::Context);
            }
            _ => panic!("expected Line row"),
        }
    }

    #[test]
    fn hunk_line_numbers_start_correctly() {
        let input = "\
@@ -5,2 +10,2 @@
 context
-deleted
+added
";
        let diff = parse_side_by_side(input);
        assert_eq!(diff.rows.len(), 2);
        match &diff.rows[0] {
            SideBySideRow::Line { left, right } => {
                assert_eq!(left.as_ref().unwrap().lineno, 5);
                assert_eq!(right.as_ref().unwrap().lineno, 10);
            }
            _ => panic!("expected Line row"),
        }
        match &diff.rows[1] {
            SideBySideRow::Line { left, right } => {
                assert_eq!(left.as_ref().unwrap().lineno, 6);
                assert_eq!(right.as_ref().unwrap().lineno, 11);
            }
            _ => panic!("expected Line row"),
        }
    }

    #[test]
    fn single_hunk_no_separator() {
        let input = "\
@@ -1,1 +1,1 @@
-old
+new
";
        let diff = parse_side_by_side(input);
        for row in &diff.rows {
            assert_ne!(*row, SideBySideRow::HunkSeparator);
        }
    }
}
