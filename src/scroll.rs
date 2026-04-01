/// Manages cursor position and scroll offset for a scrollable list.
pub struct ScrollState {
    pub cursor: usize,
    pub offset: usize,
    pub visible_rows: usize,
}

impl ScrollState {
    /// Move cursor up by one, adjusting offset if needed.
    /// Returns true if cursor changed.
    pub fn up(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        if self.cursor < self.offset {
            self.offset = self.cursor;
        }
        true
    }

    /// Move cursor down by one, adjusting offset if needed.
    /// Returns true if cursor changed.
    pub fn down(&mut self, item_count: usize) -> bool {
        if item_count == 0 || self.cursor >= item_count - 1 {
            return false;
        }
        self.cursor += 1;
        if self.visible_rows > 0 && self.cursor >= self.offset + self.visible_rows {
            self.offset = self.cursor - self.visible_rows + 1;
        }
        true
    }

    /// Move cursor to the first item.
    pub fn first(&mut self) {
        self.cursor = 0;
        self.offset = 0;
    }

    /// Move cursor to the last item.
    pub fn last(&mut self, item_count: usize) {
        if item_count == 0 {
            return;
        }
        self.cursor = item_count - 1;
        if self.visible_rows > 0 && self.cursor >= self.visible_rows {
            self.offset = self.cursor - self.visible_rows + 1;
        } else {
            self.offset = 0;
        }
    }

    /// Move cursor down by one page.
    pub fn page_down(&mut self, item_count: usize) {
        if item_count == 0 {
            return;
        }
        self.cursor = (self.cursor + self.visible_rows).min(item_count - 1);
        if self.visible_rows > 0 && self.cursor >= self.offset + self.visible_rows {
            self.offset = self.cursor - self.visible_rows + 1;
        }
    }

    /// Move cursor up by one page.
    pub fn page_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(self.visible_rows);
        if self.cursor < self.offset {
            self.offset = self.cursor;
        }
    }

    /// Returns the range of visible item indices (start inclusive, end exclusive).
    pub fn visible_range(&self, item_count: usize) -> (usize, usize) {
        let start = self.offset;
        let end = (self.offset + self.visible_rows).min(item_count);
        if start > end {
            (end, end)
        } else {
            (start, end)
        }
    }

    /// Ensure cursor and offset are valid for the given item count.
    pub fn clamp(&mut self, item_count: usize) {
        if item_count == 0 {
            self.cursor = 0;
            self.offset = 0;
            return;
        }
        if self.cursor >= item_count {
            self.cursor = item_count - 1;
        }
        if self.offset > self.cursor {
            self.offset = self.cursor;
        }
    }

    /// Reset cursor and offset to zero.
    pub fn reset(&mut self) {
        self.cursor = 0;
        self.offset = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn up_from_middle() {
        let mut s = ScrollState {
            cursor: 5,
            offset: 0,
            visible_rows: 10,
        };
        assert!(s.up());
        assert_eq!(s.cursor, 4);
        assert_eq!(s.offset, 0);
    }

    #[test]
    fn up_at_top() {
        let mut s = ScrollState {
            cursor: 0,
            offset: 0,
            visible_rows: 10,
        };
        assert!(!s.up());
        assert_eq!(s.cursor, 0);
        assert_eq!(s.offset, 0);
    }

    #[test]
    fn down_from_middle() {
        let mut s = ScrollState {
            cursor: 5,
            offset: 0,
            visible_rows: 10,
        };
        assert!(s.down(20));
        assert_eq!(s.cursor, 6);
        assert_eq!(s.offset, 0);
    }

    #[test]
    fn down_at_bottom() {
        let mut s = ScrollState {
            cursor: 9,
            offset: 0,
            visible_rows: 10,
        };
        assert!(!s.down(10));
        assert_eq!(s.cursor, 9);
        assert_eq!(s.offset, 0);
    }

    #[test]
    fn down_scrolls_offset() {
        let mut s = ScrollState {
            cursor: 9,
            offset: 0,
            visible_rows: 10,
        };
        assert!(s.down(20));
        assert_eq!(s.cursor, 10);
        assert_eq!(s.offset, 1);
    }

    #[test]
    fn down_empty_list() {
        let mut s = ScrollState {
            cursor: 0,
            offset: 0,
            visible_rows: 10,
        };
        assert!(!s.down(0));
        assert_eq!(s.cursor, 0);
    }

    #[test]
    fn up_scrolls_offset() {
        let mut s = ScrollState {
            cursor: 5,
            offset: 5,
            visible_rows: 10,
        };
        assert!(s.up());
        assert_eq!(s.cursor, 4);
        assert_eq!(s.offset, 4);
    }

    #[test]
    fn first() {
        let mut s = ScrollState {
            cursor: 15,
            offset: 10,
            visible_rows: 10,
        };
        s.first();
        assert_eq!(s.cursor, 0);
        assert_eq!(s.offset, 0);
    }

    #[test]
    fn last_with_scrolling() {
        let mut s = ScrollState {
            cursor: 0,
            offset: 0,
            visible_rows: 10,
        };
        s.last(20);
        assert_eq!(s.cursor, 19);
        assert_eq!(s.offset, 10);
    }

    #[test]
    fn last_fewer_items_than_visible() {
        let mut s = ScrollState {
            cursor: 0,
            offset: 0,
            visible_rows: 10,
        };
        s.last(5);
        assert_eq!(s.cursor, 4);
        assert_eq!(s.offset, 0);
    }

    #[test]
    fn last_empty_list() {
        let mut s = ScrollState {
            cursor: 5,
            offset: 3,
            visible_rows: 10,
        };
        s.last(0);
        assert_eq!(s.cursor, 5);
        assert_eq!(s.offset, 3);
    }

    #[test]
    fn page_down_moves_by_visible_rows() {
        let mut s = ScrollState {
            cursor: 0,
            offset: 0,
            visible_rows: 10,
        };
        s.page_down(30);
        assert_eq!(s.cursor, 10);
        assert_eq!(s.offset, 1);
    }

    #[test]
    fn page_down_clamps_to_last() {
        let mut s = ScrollState {
            cursor: 15,
            offset: 10,
            visible_rows: 10,
        };
        s.page_down(20);
        assert_eq!(s.cursor, 19);
        assert_eq!(s.offset, 10);
    }

    #[test]
    fn page_up_moves_by_visible_rows() {
        let mut s = ScrollState {
            cursor: 15,
            offset: 10,
            visible_rows: 10,
        };
        s.page_up();
        assert_eq!(s.cursor, 5);
        assert_eq!(s.offset, 5);
    }

    #[test]
    fn page_up_clamps_to_first() {
        let mut s = ScrollState {
            cursor: 3,
            offset: 0,
            visible_rows: 10,
        };
        s.page_up();
        assert_eq!(s.cursor, 0);
        assert_eq!(s.offset, 0);
    }

    #[test]
    fn visible_range_normal() {
        let s = ScrollState {
            cursor: 5,
            offset: 0,
            visible_rows: 10,
        };
        assert_eq!(s.visible_range(20), (0, 10));
    }

    #[test]
    fn visible_range_scrolled() {
        let s = ScrollState {
            cursor: 15,
            offset: 10,
            visible_rows: 10,
        };
        assert_eq!(s.visible_range(20), (10, 20));
    }

    #[test]
    fn visible_range_fewer_items() {
        let s = ScrollState {
            cursor: 0,
            offset: 0,
            visible_rows: 10,
        };
        assert_eq!(s.visible_range(5), (0, 5));
    }

    #[test]
    fn visible_range_empty() {
        let s = ScrollState {
            cursor: 0,
            offset: 0,
            visible_rows: 10,
        };
        assert_eq!(s.visible_range(0), (0, 0));
    }

    #[test]
    fn clamp_cursor_beyond_count() {
        let mut s = ScrollState {
            cursor: 15,
            offset: 10,
            visible_rows: 10,
        };
        s.clamp(10);
        assert_eq!(s.cursor, 9);
    }

    #[test]
    fn clamp_empty_list() {
        let mut s = ScrollState {
            cursor: 5,
            offset: 3,
            visible_rows: 10,
        };
        s.clamp(0);
        assert_eq!(s.cursor, 0);
        assert_eq!(s.offset, 0);
    }

    #[test]
    fn clamp_offset_beyond_cursor() {
        let mut s = ScrollState {
            cursor: 2,
            offset: 5,
            visible_rows: 10,
        };
        s.clamp(10);
        assert_eq!(s.cursor, 2);
        assert_eq!(s.offset, 2);
    }

    #[test]
    fn clamp_already_valid() {
        let mut s = ScrollState {
            cursor: 5,
            offset: 3,
            visible_rows: 10,
        };
        s.clamp(20);
        assert_eq!(s.cursor, 5);
        assert_eq!(s.offset, 3);
    }

    #[test]
    fn reset_clears_position() {
        let mut s = ScrollState {
            cursor: 15,
            offset: 10,
            visible_rows: 10,
        };
        s.reset();
        assert_eq!(s.cursor, 0);
        assert_eq!(s.offset, 0);
        assert_eq!(s.visible_rows, 10);
    }
}
