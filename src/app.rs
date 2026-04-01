use crossterm::event::KeyCode;
use ratatui::text::Text;

use crate::git::{Commit, FileEntry};
use crate::scroll::ScrollState;
use crate::tree::{FileTree, VisibleItem, VisibleItemKind};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Panel {
    CommitList,
    FileList,
    Diff,
}

#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    Quit,
    Render,
    SelectCommit(usize),
    LoadDiff(usize),
}

pub struct App<'a> {
    pub commits: Vec<Commit>,
    pub commit_scroll: ScrollState,
    pub files: Vec<FileEntry>,
    pub tree: FileTree,
    pub visible_items: Vec<VisibleItem>,
    pub file_scroll: ScrollState,
    pub diff_scroll: ScrollState,
    pub active_panel: Panel,
    pub diff_content: Option<Text<'a>>,
    pub diff_line_count: usize,
    last_selected_commit: Option<usize>,
    last_selected_file: Option<usize>,
}

impl<'a> App<'a> {
    pub fn new(commits: Vec<Commit>) -> Self {
        App {
            commits,
            commit_scroll: ScrollState {
                cursor: 0,
                offset: 0,
                visible_rows: 0,
            },
            files: Vec::new(),
            tree: FileTree::build(&[]),
            visible_items: Vec::new(),
            file_scroll: ScrollState {
                cursor: 0,
                offset: 0,
                visible_rows: 0,
            },
            diff_scroll: ScrollState {
                cursor: 0,
                offset: 0,
                visible_rows: 0,
            },
            active_panel: Panel::CommitList,
            diff_content: None,
            diff_line_count: 0,
            last_selected_commit: None,
            last_selected_file: None,
        }
    }

    pub fn selected_commit(&self) -> Option<&Commit> {
        self.commits.get(self.commit_scroll.cursor)
    }

    /// Set files for the selected commit. Rebuilds tree and visible items.
    /// Returns the selected file entry index if a file is selected.
    pub fn set_commit_files(&mut self, files: Vec<FileEntry>) -> Option<usize> {
        self.tree = FileTree::build(&files);
        self.visible_items = self.tree.visible_items(&files);
        self.files = files;
        self.file_scroll.reset();
        self.diff_content = None;
        self.diff_line_count = 0;
        self.diff_scroll.reset();
        self.last_selected_file = None;

        // Return the file entry index of the first visible file
        self.visible_items.first().and_then(|item| match &item.kind {
            VisibleItemKind::File { entry_index, .. } => Some(*entry_index),
            VisibleItemKind::Directory { .. } => None,
        })
    }

    pub fn selected_file(&self) -> Option<&FileEntry> {
        let item = self.visible_items.get(self.file_scroll.cursor)?;
        match &item.kind {
            VisibleItemKind::File { entry_index, .. } => self.files.get(*entry_index),
            VisibleItemKind::Directory { .. } => None,
        }
    }

    pub fn set_diff_content(&mut self, content: Text<'a>, line_count: usize) {
        self.diff_content = Some(content);
        self.diff_line_count = line_count;
        self.diff_scroll.reset();
    }

    /// Returns an action to perform after handling the key.
    pub fn handle_key(&mut self, key: KeyCode) -> Action {
        match self.active_panel {
            Panel::CommitList => self.handle_commit_list_key(key),
            Panel::FileList => self.handle_file_list_key(key),
            Panel::Diff => self.handle_diff_key(key),
        }
    }

    fn handle_commit_list_key(&mut self, key: KeyCode) -> Action {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
            KeyCode::Char('j') | KeyCode::Down => {
                self.commit_scroll.down(self.commits.len());
                self.check_commit_selection_changed()
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.commit_scroll.up();
                self.check_commit_selection_changed()
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.commit_scroll.first();
                self.check_commit_selection_changed()
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.commit_scroll.last(self.commits.len());
                self.check_commit_selection_changed()
            }
            KeyCode::Tab => {
                if !self.files.is_empty() {
                    self.active_panel = Panel::FileList;
                }
                Action::Render
            }
            _ => Action::None,
        }
    }

    fn check_commit_selection_changed(&mut self) -> Action {
        let current = Some(self.commit_scroll.cursor);
        if current != self.last_selected_commit {
            self.last_selected_commit = current;
            if let Some(idx) = current {
                return Action::SelectCommit(idx);
            }
        }
        Action::Render
    }

    fn handle_file_list_key(&mut self, key: KeyCode) -> Action {
        match key {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Esc => {
                self.active_panel = Panel::CommitList;
                Action::Render
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.file_scroll.down(self.visible_items.len());
                self.check_selection_changed()
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.file_scroll.up();
                self.check_selection_changed()
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.file_scroll.first();
                self.check_selection_changed()
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.file_scroll.last(self.visible_items.len());
                self.check_selection_changed()
            }
            KeyCode::Enter => {
                let cursor = self.file_scroll.cursor;
                let is_dir = self.visible_items.get(cursor)
                    .map_or(false, |item| matches!(item.kind, VisibleItemKind::Directory { .. }));
                if is_dir {
                    self.tree.toggle_at_visible(cursor);
                    self.visible_items = self.tree.visible_items(&self.files);
                    self.file_scroll.clamp(self.visible_items.len());
                }
                Action::Render
            }
            KeyCode::Tab => {
                if self.diff_content.is_some() {
                    self.active_panel = Panel::Diff;
                }
                Action::Render
            }
            _ => Action::None,
        }
    }

    fn handle_diff_key(&mut self, key: KeyCode) -> Action {
        match key {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Tab => {
                self.active_panel = Panel::FileList;
                Action::Render
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.diff_scroll.down(self.diff_line_count);
                Action::Render
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.diff_scroll.up();
                Action::Render
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.diff_scroll.first();
                Action::Render
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.diff_scroll.last(self.diff_line_count);
                Action::Render
            }
            KeyCode::PageDown => {
                self.diff_scroll.page_down(self.diff_line_count);
                Action::Render
            }
            KeyCode::PageUp => {
                self.diff_scroll.page_up();
                Action::Render
            }
            _ => Action::None,
        }
    }

    fn check_selection_changed(&mut self) -> Action {
        let current_file = self.visible_items.get(self.file_scroll.cursor).and_then(|item| {
            match &item.kind {
                VisibleItemKind::File { entry_index, .. } => Some(*entry_index),
                VisibleItemKind::Directory { .. } => None,
            }
        });

        if current_file != self.last_selected_file {
            self.last_selected_file = current_file;
            if let Some(entry_index) = current_file {
                return Action::LoadDiff(entry_index);
            } else {
                // Moved to a directory — clear diff
                self.diff_content = None;
                self.diff_line_count = 0;
            }
        }
        Action::Render
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{Commit, FileStatus};

    fn sample_commits() -> Vec<Commit> {
        vec![
            Commit {
                hash: "abc1234567890".into(),
                subject: "first commit".into(),
                parent_hash: Some("def5678".into()),
            },
            Commit {
                hash: "bbb2222222222".into(),
                subject: "second commit".into(),
                parent_hash: Some("abc1234567890".into()),
            },
            Commit {
                hash: "ccc3333333333".into(),
                subject: "third commit".into(),
                parent_hash: Some("bbb2222222222".into()),
            },
        ]
    }

    // Tree structure for sample_files():
    //   0: ▼ src/        (directory)
    //   1:   A main.rs   (entry_index 0)
    //   2:   M lib.rs    (entry_index 1)
    //   3: D old.rs      (entry_index 2)
    fn sample_files() -> Vec<FileEntry> {
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
                path: "old.rs".into(),
                status: FileStatus::Deleted,
            },
        ]
    }

    fn app_with_files() -> App<'static> {
        let mut app = App::new(sample_commits());
        app.set_commit_files(sample_files());
        app.last_selected_commit = Some(0);
        app
    }

    #[test]
    fn new_app_starts_on_commit_list() {
        let app = App::new(sample_commits());
        assert_eq!(app.active_panel, Panel::CommitList);
        assert_eq!(app.commit_scroll.cursor, 0);
    }

    #[test]
    fn new_app_empty_commits() {
        let app = App::new(vec![]);
        assert_eq!(app.active_panel, Panel::CommitList);
        assert!(app.selected_commit().is_none());
        assert!(app.selected_file().is_none());
    }

    // --- Commit list tests ---

    #[test]
    fn commit_list_j_moves_down() {
        let mut app = App::new(sample_commits());
        app.commit_scroll.visible_rows = 10;
        let action = app.handle_key(KeyCode::Char('j'));
        assert_eq!(app.commit_scroll.cursor, 1);
        assert_eq!(action, Action::SelectCommit(1));
    }

    #[test]
    fn commit_list_k_moves_up() {
        let mut app = App::new(sample_commits());
        app.commit_scroll.visible_rows = 10;
        app.commit_scroll.cursor = 1;
        app.last_selected_commit = Some(1);
        let action = app.handle_key(KeyCode::Char('k'));
        assert_eq!(app.commit_scroll.cursor, 0);
        assert_eq!(action, Action::SelectCommit(0));
    }

    #[test]
    fn commit_list_g_jumps_first() {
        let mut app = App::new(sample_commits());
        app.commit_scroll.visible_rows = 10;
        app.commit_scroll.cursor = 2;
        app.last_selected_commit = Some(2);
        let action = app.handle_key(KeyCode::Char('g'));
        assert_eq!(app.commit_scroll.cursor, 0);
        assert_eq!(action, Action::SelectCommit(0));
    }

    #[test]
    fn commit_list_shift_g_jumps_last() {
        let mut app = App::new(sample_commits());
        app.commit_scroll.visible_rows = 10;
        let action = app.handle_key(KeyCode::Char('G'));
        assert_eq!(app.commit_scroll.cursor, 2);
        assert_eq!(action, Action::SelectCommit(2));
    }

    #[test]
    fn commit_list_tab_switches_to_file_list() {
        let mut app = App::new(sample_commits());
        app.set_commit_files(sample_files());
        let action = app.handle_key(KeyCode::Tab);
        assert_eq!(action, Action::Render);
        assert_eq!(app.active_panel, Panel::FileList);
    }

    #[test]
    fn commit_list_tab_stays_when_no_files() {
        let mut app = App::new(sample_commits());
        let action = app.handle_key(KeyCode::Tab);
        assert_eq!(action, Action::Render);
        assert_eq!(app.active_panel, Panel::CommitList);
    }

    #[test]
    fn commit_list_q_quits() {
        let mut app = App::new(sample_commits());
        assert_eq!(app.handle_key(KeyCode::Char('q')), Action::Quit);
    }

    #[test]
    fn commit_list_esc_quits() {
        let mut app = App::new(sample_commits());
        assert_eq!(app.handle_key(KeyCode::Esc), Action::Quit);
    }

    #[test]
    fn file_list_esc_goes_to_commit_list() {
        let mut app = app_with_files();
        app.active_panel = Panel::FileList;
        let action = app.handle_key(KeyCode::Esc);
        assert_eq!(action, Action::Render);
        assert_eq!(app.active_panel, Panel::CommitList);
    }

    #[test]
    fn set_commit_files_rebuilds_tree() {
        let mut app = App::new(sample_commits());
        assert!(app.visible_items.is_empty());
        app.set_commit_files(sample_files());
        assert_eq!(app.visible_items.len(), 4); // dir + 3 files
        assert_eq!(app.files.len(), 3);
    }

    #[test]
    fn set_commit_files_resets_scroll() {
        let mut app = App::new(sample_commits());
        app.file_scroll.cursor = 5;
        app.file_scroll.offset = 3;
        app.diff_scroll.cursor = 10;
        app.diff_scroll.offset = 5;
        app.set_commit_files(sample_files());
        assert_eq!(app.file_scroll.cursor, 0);
        assert_eq!(app.file_scroll.offset, 0);
        assert_eq!(app.diff_scroll.cursor, 0);
        assert_eq!(app.diff_scroll.offset, 0);
        assert!(app.diff_content.is_none());
    }

    // --- File list tests (updated for new App::new) ---

    #[test]
    fn j_moves_down_through_visible_items() {
        let mut app = app_with_files();
        app.active_panel = Panel::FileList;
        app.file_scroll.visible_rows = 10;
        // cursor 0 (dir) → 1 (file, entry_index 0)
        let action = app.handle_key(KeyCode::Char('j'));
        assert_eq!(app.file_scroll.cursor, 1);
        assert_eq!(action, Action::LoadDiff(0));
    }

    #[test]
    fn k_moves_up() {
        let mut app = app_with_files();
        app.active_panel = Panel::FileList;
        app.file_scroll.visible_rows = 10;
        app.handle_key(KeyCode::Char('j')); // 0→1
        app.handle_key(KeyCode::Char('j')); // 1→2
        let action = app.handle_key(KeyCode::Char('k')); // 2→1
        assert_eq!(app.file_scroll.cursor, 1);
        assert_eq!(action, Action::LoadDiff(0));
    }

    #[test]
    fn j_at_bottom_stays() {
        let mut app = app_with_files();
        app.active_panel = Panel::FileList;
        app.file_scroll.visible_rows = 10;
        app.handle_key(KeyCode::Char('j')); // 1
        app.handle_key(KeyCode::Char('j')); // 2
        app.handle_key(KeyCode::Char('j')); // 3
        let action = app.handle_key(KeyCode::Char('j')); // stays at 3
        assert_eq!(action, Action::Render);
        assert_eq!(app.file_scroll.cursor, 3);
    }

    #[test]
    fn file_list_q_quits() {
        let mut app = app_with_files();
        app.active_panel = Panel::FileList;
        assert_eq!(app.handle_key(KeyCode::Char('q')), Action::Quit);
    }

    #[test]
    fn tab_switches_to_diff_when_content_loaded() {
        let mut app = app_with_files();
        app.active_panel = Panel::FileList;
        app.set_diff_content(Text::raw("diff"), 1);
        let action = app.handle_key(KeyCode::Tab);
        assert_eq!(action, Action::Render);
        assert_eq!(app.active_panel, Panel::Diff);
    }

    #[test]
    fn tab_stays_on_file_list_when_no_diff() {
        let mut app = app_with_files();
        app.active_panel = Panel::FileList;
        let action = app.handle_key(KeyCode::Tab);
        assert_eq!(action, Action::Render);
        assert_eq!(app.active_panel, Panel::FileList);
    }

    #[test]
    fn diff_panel_tab_returns_to_file_list() {
        let mut app = app_with_files();
        app.set_diff_content(Text::raw("diff"), 1);
        app.active_panel = Panel::Diff;
        let action = app.handle_key(KeyCode::Tab);
        assert_eq!(action, Action::Render);
        assert_eq!(app.active_panel, Panel::FileList);
    }

    #[test]
    fn diff_panel_j_scrolls() {
        let mut app = app_with_files();
        app.set_diff_content(Text::raw("line1\nline2\nline3"), 3);
        app.diff_scroll.visible_rows = 2;
        app.active_panel = Panel::Diff;
        let action = app.handle_key(KeyCode::Char('j'));
        assert_eq!(action, Action::Render);
        assert_eq!(app.diff_scroll.cursor, 1);
    }

    #[test]
    fn diff_panel_esc_returns_to_file_list() {
        let mut app = app_with_files();
        app.active_panel = Panel::Diff;
        let action = app.handle_key(KeyCode::Esc);
        assert_eq!(action, Action::Render);
        assert_eq!(app.active_panel, Panel::FileList);
    }

    #[test]
    fn set_diff_content_resets_scroll() {
        let mut app = app_with_files();
        app.diff_scroll.cursor = 10;
        app.diff_scroll.offset = 5;
        app.set_diff_content(Text::raw("new"), 1);
        assert_eq!(app.diff_scroll.cursor, 0);
        assert_eq!(app.diff_scroll.offset, 0);
    }

    #[test]
    fn g_jumps_to_first_visible_item() {
        let mut app = app_with_files();
        app.active_panel = Panel::FileList;
        app.file_scroll.visible_rows = 10;
        app.handle_key(KeyCode::Char('j')); // 1 (file)
        app.handle_key(KeyCode::Char('j')); // 2 (file)
        app.handle_key(KeyCode::Char('j')); // 3 (file)
        let action = app.handle_key(KeyCode::Char('g')); // back to 0 (dir)
        assert_eq!(app.file_scroll.cursor, 0);
        assert_eq!(action, Action::Render);
        assert!(app.diff_content.is_none());
    }

    #[test]
    fn shift_g_jumps_to_last_visible_item() {
        let mut app = app_with_files();
        app.active_panel = Panel::FileList;
        app.file_scroll.visible_rows = 10;
        let action = app.handle_key(KeyCode::Char('G')); // jump to 3 (old.rs)
        assert_eq!(app.file_scroll.cursor, 3);
        assert_eq!(action, Action::LoadDiff(2));
    }

    #[test]
    fn enter_toggles_directory() {
        let mut app = app_with_files();
        app.active_panel = Panel::FileList;
        app.file_scroll.visible_rows = 10;
        assert_eq!(app.visible_items.len(), 4);
        let action = app.handle_key(KeyCode::Enter);
        assert_eq!(action, Action::Render);
        assert_eq!(app.visible_items.len(), 2);
    }

    #[test]
    fn enter_on_file_is_noop() {
        let mut app = app_with_files();
        app.active_panel = Panel::FileList;
        app.file_scroll.visible_rows = 10;
        app.handle_key(KeyCode::Char('j')); // cursor to 1 (file)
        let before = app.visible_items.len();
        let action = app.handle_key(KeyCode::Enter);
        assert_eq!(action, Action::Render);
        assert_eq!(app.visible_items.len(), before);
    }

    #[test]
    fn selected_file_on_directory_returns_none() {
        let app = app_with_files();
        // cursor at 0 = directory
        assert!(app.selected_file().is_none());
    }

    #[test]
    fn selected_file_on_file_returns_entry() {
        let mut app = app_with_files();
        app.active_panel = Panel::FileList;
        app.file_scroll.visible_rows = 10;
        app.handle_key(KeyCode::Char('j')); // cursor to 1 (main.rs)
        let file = app.selected_file().unwrap();
        assert_eq!(file.path, "src/main.rs");
    }

    #[test]
    fn moving_to_directory_clears_diff() {
        let mut app = app_with_files();
        app.active_panel = Panel::FileList;
        app.file_scroll.visible_rows = 10;
        app.handle_key(KeyCode::Char('j')); // cursor 1, loads diff
        app.set_diff_content(Text::raw("diff"), 1);
        assert!(app.diff_content.is_some());
        app.handle_key(KeyCode::Char('k')); // cursor 0, directory
        assert!(app.diff_content.is_none());
    }

    #[test]
    fn navigation_skips_collapsed_children() {
        let mut app = app_with_files();
        app.active_panel = Panel::FileList;
        app.file_scroll.visible_rows = 10;
        app.handle_key(KeyCode::Enter); // collapse src/
        let action = app.handle_key(KeyCode::Char('j')); // 0→1 (old.rs)
        assert_eq!(app.file_scroll.cursor, 1);
        assert_eq!(action, Action::LoadDiff(2));
    }
}
