use crossterm::event::KeyCode;
use ratatui::text::Text;

use crate::git::FileEntry;
use crate::scroll::ScrollState;

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
    pub files: Vec<FileEntry>,
    pub file_scroll: ScrollState,
    pub diff_scroll: ScrollState,
    pub active_panel: Panel,
    pub diff_content: Option<Text<'a>>,
    pub diff_line_count: usize,
    pub description: String,
    last_selected: Option<usize>,
}

impl<'a> App<'a> {
    pub fn new(files: Vec<FileEntry>, description: String) -> Self {
        let initial_selected = if files.is_empty() { None } else { Some(0) };
        App {
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
            files,
            active_panel: Panel::FileList,
            diff_content: None,
            diff_line_count: 0,
            description,
            last_selected: initial_selected,
        }
    }

    pub fn selected_file(&self) -> Option<&FileEntry> {
        self.files.get(self.file_scroll.cursor)
    }

    pub fn set_diff_content(&mut self, content: Text<'a>, line_count: usize) {
        self.diff_content = Some(content);
        self.diff_line_count = line_count;
        self.diff_scroll.reset();
    }

    /// Returns an action to perform after handling the key.
    /// If the selected file changed, returns LoadDiff with the new index.
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
                self.file_scroll.down(self.files.len());
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
                self.file_scroll.last(self.files.len());
                self.check_selection_changed()
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
        let current = Some(self.file_scroll.cursor);
        if current != self.last_selected {
            self.last_selected = current;
            if let Some(idx) = current {
                return Action::LoadDiff(idx);
            }
        }
        Action::Render
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::FileStatus;

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

    #[test]
    fn new_app_selects_first_file() {
        let app = App::new(sample_files(), "test".into());
        assert_eq!(app.file_scroll.cursor, 0);
        assert_eq!(app.active_panel, Panel::FileList);
        assert_eq!(app.selected_file().unwrap().path, "src/main.rs");
    }

    #[test]
    fn new_app_empty_files() {
        let app = App::new(vec![], "test".into());
        assert!(app.selected_file().is_none());
    }

    #[test]
    fn j_moves_selection_down() {
        let mut app = App::new(sample_files(), "test".into());
        app.file_scroll.visible_rows = 10;
        // First file is auto-selected, so moving down should load the second
        let action = app.handle_key(KeyCode::Char('j'));
        assert_eq!(action, Action::LoadDiff(1));
        assert_eq!(app.file_scroll.cursor, 1);
    }

    #[test]
    fn k_moves_selection_up() {
        let mut app = App::new(sample_files(), "test".into());
        app.file_scroll.visible_rows = 10;
        app.handle_key(KeyCode::Char('j')); // move to 1
        let action = app.handle_key(KeyCode::Char('k'));
        assert_eq!(action, Action::LoadDiff(0));
        assert_eq!(app.file_scroll.cursor, 0);
    }

    #[test]
    fn j_at_bottom_stays() {
        let mut app = App::new(sample_files(), "test".into());
        app.file_scroll.visible_rows = 10;
        app.handle_key(KeyCode::Char('j')); // 1
        app.handle_key(KeyCode::Char('j')); // 2
        let action = app.handle_key(KeyCode::Char('j')); // stays at 2
        assert_eq!(action, Action::Render); // no change = Render, not LoadDiff
        assert_eq!(app.file_scroll.cursor, 2);
    }

    #[test]
    fn q_quits() {
        let mut app = App::new(sample_files(), "test".into());
        assert_eq!(app.handle_key(KeyCode::Char('q')), Action::Quit);
    }

    #[test]
    fn esc_quits() {
        let mut app = App::new(sample_files(), "test".into());
        assert_eq!(app.handle_key(KeyCode::Esc), Action::Quit);
    }

    #[test]
    fn tab_switches_to_diff_when_content_loaded() {
        let mut app = App::new(sample_files(), "test".into());
        app.set_diff_content(Text::raw("diff"), 1);
        let action = app.handle_key(KeyCode::Tab);
        assert_eq!(action, Action::Render);
        assert_eq!(app.active_panel, Panel::Diff);
    }

    #[test]
    fn tab_stays_on_file_list_when_no_diff() {
        let mut app = App::new(sample_files(), "test".into());
        let action = app.handle_key(KeyCode::Tab);
        assert_eq!(action, Action::Render);
        assert_eq!(app.active_panel, Panel::FileList);
    }

    #[test]
    fn diff_panel_tab_returns_to_file_list() {
        let mut app = App::new(sample_files(), "test".into());
        app.set_diff_content(Text::raw("diff"), 1);
        app.active_panel = Panel::Diff;
        let action = app.handle_key(KeyCode::Tab);
        assert_eq!(action, Action::Render);
        assert_eq!(app.active_panel, Panel::FileList);
    }

    #[test]
    fn diff_panel_j_scrolls() {
        let mut app = App::new(sample_files(), "test".into());
        app.set_diff_content(Text::raw("line1\nline2\nline3"), 3);
        app.diff_scroll.visible_rows = 2;
        app.active_panel = Panel::Diff;
        let action = app.handle_key(KeyCode::Char('j'));
        assert_eq!(action, Action::Render);
        assert_eq!(app.diff_scroll.cursor, 1);
    }

    #[test]
    fn diff_panel_esc_returns_to_file_list() {
        let mut app = App::new(sample_files(), "test".into());
        app.active_panel = Panel::Diff;
        let action = app.handle_key(KeyCode::Esc);
        assert_eq!(action, Action::Render);
        assert_eq!(app.active_panel, Panel::FileList);
    }

    #[test]
    fn set_diff_content_resets_scroll() {
        let mut app = App::new(sample_files(), "test".into());
        app.diff_scroll.cursor = 10;
        app.diff_scroll.offset = 5;
        app.set_diff_content(Text::raw("new"), 1);
        assert_eq!(app.diff_scroll.cursor, 0);
        assert_eq!(app.diff_scroll.offset, 0);
    }

    #[test]
    fn g_jumps_to_first_file() {
        let mut app = App::new(sample_files(), "test".into());
        app.file_scroll.visible_rows = 10;
        app.handle_key(KeyCode::Char('j'));
        app.handle_key(KeyCode::Char('j'));
        let action = app.handle_key(KeyCode::Char('g'));
        assert_eq!(action, Action::LoadDiff(0));
        assert_eq!(app.file_scroll.cursor, 0);
    }

    #[test]
    fn shift_g_jumps_to_last_file() {
        let mut app = App::new(sample_files(), "test".into());
        app.file_scroll.visible_rows = 10;
        let action = app.handle_key(KeyCode::Char('G'));
        assert_eq!(action, Action::LoadDiff(2));
        assert_eq!(app.file_scroll.cursor, 2);
    }
}
