use crossterm::event::{KeyCode, KeyEvent};
use ratatui::text::Text;

use crate::git::FileEntry;
use crate::scroll::ScrollState;
use crate::tree::{FileTree, VisibleItem, VisibleItemKind};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Panel {
    FileList,
    Diff,
}

#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    Quit,
    Render,
    LoadDiff(usize),
}

pub struct App<'a> {
    pub diff_args: Vec<String>,
    pub files: Vec<FileEntry>,
    pub tree: FileTree,
    pub visible_items: Vec<VisibleItem>,
    pub file_scroll: ScrollState,
    pub diff_scroll: ScrollState,
    pub active_panel: Panel,
    pub diff_content: Option<Text<'a>>,
    pub diff_line_count: usize,
    last_selected_file: Option<usize>,
}

impl<'a> App<'a> {
    pub fn new(files: Vec<FileEntry>, diff_args: Vec<String>) -> Self {
        let tree = FileTree::build(&files);
        let visible_items = tree.visible_items(&files);
        App {
            diff_args,
            files,
            tree,
            visible_items,
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
            active_panel: Panel::FileList,
            diff_content: None,
            diff_line_count: 0,
            last_selected_file: None,
        }
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

    /// Top-level input handler.
    pub fn handle_key_event(&mut self, event: KeyEvent) -> Action {
        self.handle_key(event.code)
    }

    /// Returns an action to perform after handling the key.
    pub fn handle_key(&mut self, key: KeyCode) -> Action {
        match self.active_panel {
            Panel::FileList => self.handle_file_list_key(key),
            Panel::Diff => self.handle_diff_key(key),
        }
    }

    fn handle_file_list_key(&mut self, key: KeyCode) -> Action {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
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
                } else if self.diff_content.is_some() {
                    self.active_panel = Panel::Diff;
                }
                Action::Render
            }
            KeyCode::Tab => {
                if self.diff_content.is_some() {
                    self.active_panel = Panel::Diff;
                }
                Action::Render
            }
            KeyCode::Char('J') => {
                self.diff_scroll.down(self.diff_line_count);
                Action::Render
            }
            KeyCode::Char('K') => {
                self.diff_scroll.up();
                Action::Render
            }
            KeyCode::PageDown => {
                self.file_scroll.page_down(self.visible_items.len());
                self.check_selection_changed()
            }
            KeyCode::PageUp => {
                self.file_scroll.page_up();
                self.check_selection_changed()
            }
            _ => Action::None,
        }
    }

    fn handle_diff_key(&mut self, key: KeyCode) -> Action {
        match key {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Esc | KeyCode::Tab | KeyCode::BackTab => {
                self.active_panel = Panel::FileList;
                Action::Render
            }
            KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Down => {
                self.diff_scroll.down(self.diff_line_count);
                Action::Render
            }
            KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Up => {
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
    use crate::git::FileStatus;

    fn dummy_diff_args() -> Vec<String> {
        vec!["diff".into(), "--no-ext-diff".into()]
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
        App::new(sample_files(), dummy_diff_args())
    }

    // --- App::new tests ---

    #[test]
    fn new_app_starts_on_file_list() {
        let app = app_with_files();
        assert_eq!(app.active_panel, Panel::FileList);
        assert_eq!(app.file_scroll.cursor, 0);
    }

    #[test]
    fn new_app_empty_files() {
        let app = App::new(vec![], dummy_diff_args());
        assert_eq!(app.active_panel, Panel::FileList);
        assert!(app.visible_items.is_empty());
        assert!(app.selected_file().is_none());
    }

    #[test]
    fn new_app_builds_tree() {
        let app = app_with_files();
        assert_eq!(app.visible_items.len(), 4); // dir + 3 files
        assert_eq!(app.files.len(), 3);
    }

    // --- File list tests ---

    #[test]
    fn j_moves_down_through_visible_items() {
        let mut app = app_with_files();
        app.file_scroll.visible_rows = 10;
        // cursor 0 (dir) → 1 (file, entry_index 0)
        let action = app.handle_key(KeyCode::Char('j'));
        assert_eq!(app.file_scroll.cursor, 1);
        assert_eq!(action, Action::LoadDiff(0));
    }

    #[test]
    fn k_moves_up() {
        let mut app = app_with_files();
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
        assert_eq!(app.handle_key(KeyCode::Char('q')), Action::Quit);
    }

    #[test]
    fn file_list_esc_quits() {
        let mut app = app_with_files();
        assert_eq!(app.handle_key(KeyCode::Esc), Action::Quit);
    }

    #[test]
    fn file_list_backtab_is_noop() {
        let mut app = app_with_files();
        let action = app.handle_key(KeyCode::BackTab);
        assert_eq!(action, Action::None);
        assert_eq!(app.active_panel, Panel::FileList);
    }

    #[test]
    fn tab_switches_to_diff_when_content_loaded() {
        let mut app = app_with_files();
        app.set_diff_content(Text::raw("diff"), 1);
        let action = app.handle_key(KeyCode::Tab);
        assert_eq!(action, Action::Render);
        assert_eq!(app.active_panel, Panel::Diff);
    }

    #[test]
    fn tab_stays_on_file_list_when_no_diff() {
        let mut app = app_with_files();
        let action = app.handle_key(KeyCode::Tab);
        assert_eq!(action, Action::Render);
        assert_eq!(app.active_panel, Panel::FileList);
    }

    #[test]
    fn g_jumps_to_first_visible_item() {
        let mut app = app_with_files();
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
        app.file_scroll.visible_rows = 10;
        let action = app.handle_key(KeyCode::Char('G')); // jump to 3 (old.rs)
        assert_eq!(app.file_scroll.cursor, 3);
        assert_eq!(action, Action::LoadDiff(2));
    }

    #[test]
    fn enter_toggles_directory() {
        let mut app = app_with_files();
        app.file_scroll.visible_rows = 10;
        assert_eq!(app.visible_items.len(), 4);
        let action = app.handle_key(KeyCode::Enter);
        assert_eq!(action, Action::Render);
        assert_eq!(app.visible_items.len(), 2);
    }

    #[test]
    fn enter_on_file_is_noop() {
        let mut app = app_with_files();
        app.file_scroll.visible_rows = 10;
        app.handle_key(KeyCode::Char('j')); // cursor to 1 (file)
        let before = app.visible_items.len();
        let action = app.handle_key(KeyCode::Enter);
        assert_eq!(action, Action::Render);
        assert_eq!(app.visible_items.len(), before);
    }

    #[test]
    fn file_list_enter_on_file_switches_to_diff() {
        let mut app = app_with_files();
        app.file_scroll.visible_rows = 10;
        app.handle_key(KeyCode::Char('j')); // cursor to file
        app.set_diff_content(Text::raw("diff"), 1);
        let action = app.handle_key(KeyCode::Enter);
        assert_eq!(action, Action::Render);
        assert_eq!(app.active_panel, Panel::Diff);
    }

    #[test]
    fn file_list_enter_on_file_stays_when_no_diff() {
        let mut app = app_with_files();
        app.file_scroll.visible_rows = 10;
        app.handle_key(KeyCode::Char('j')); // cursor to file
        let action = app.handle_key(KeyCode::Enter);
        assert_eq!(action, Action::Render);
        assert_eq!(app.active_panel, Panel::FileList);
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
        app.file_scroll.visible_rows = 10;
        app.handle_key(KeyCode::Char('j')); // cursor to 1 (main.rs)
        let file = app.selected_file().unwrap();
        assert_eq!(file.path, "src/main.rs");
    }

    #[test]
    fn moving_to_directory_clears_diff() {
        let mut app = app_with_files();
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
        app.file_scroll.visible_rows = 10;
        app.handle_key(KeyCode::Enter); // collapse src/
        let action = app.handle_key(KeyCode::Char('j')); // 0→1 (old.rs)
        assert_eq!(app.file_scroll.cursor, 1);
        assert_eq!(action, Action::LoadDiff(2));
    }

    #[test]
    fn file_list_shift_j_scrolls_diff() {
        let mut app = app_with_files();
        app.set_diff_content(Text::raw("line1\nline2\nline3"), 3);
        app.diff_scroll.visible_rows = 10;
        let action = app.handle_key(KeyCode::Char('J'));
        assert_eq!(action, Action::Render);
        assert_eq!(app.diff_scroll.cursor, 1);
        assert_eq!(app.active_panel, Panel::FileList);
    }

    #[test]
    fn file_list_page_down() {
        let mut app = app_with_files();
        app.file_scroll.visible_rows = 2;
        let action = app.handle_key(KeyCode::PageDown);
        assert_eq!(app.file_scroll.cursor, 2);
        assert_eq!(action, Action::LoadDiff(1));
    }

    #[test]
    fn file_list_page_up() {
        let mut app = app_with_files();
        app.file_scroll.visible_rows = 2;
        app.file_scroll.cursor = 3;
        app.last_selected_file = Some(2);
        let action = app.handle_key(KeyCode::PageUp);
        assert_eq!(app.file_scroll.cursor, 1);
        assert_eq!(action, Action::LoadDiff(0));
    }

    // --- Diff panel tests ---

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
    fn diff_panel_q_quits() {
        let mut app = app_with_files();
        app.active_panel = Panel::Diff;
        assert_eq!(app.handle_key(KeyCode::Char('q')), Action::Quit);
    }

    #[test]
    fn diff_backtab_goes_to_file_list() {
        let mut app = app_with_files();
        app.active_panel = Panel::Diff;
        let action = app.handle_key(KeyCode::BackTab);
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
}
