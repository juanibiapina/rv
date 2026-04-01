use crate::git::{FileEntry, FileStatus};

pub struct FileTree {
    pub roots: Vec<TreeNode>,
}

pub struct TreeNode {
    pub name: String,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
    pub entry_index: Option<usize>, // Some = file, None = directory
}

impl TreeNode {
    pub fn is_dir(&self) -> bool {
        self.entry_index.is_none()
    }
}

#[derive(Debug, PartialEq)]
pub enum VisibleItemKind {
    Directory { expanded: bool },
    File { status: FileStatus, entry_index: usize },
}

#[derive(Debug, PartialEq)]
pub struct VisibleItem {
    pub depth: usize,
    pub name: String,
    pub kind: VisibleItemKind,
}

impl FileTree {
    pub fn build(files: &[FileEntry]) -> Self {
        let mut roots = Vec::new();
        for (idx, file) in files.iter().enumerate() {
            let parts: Vec<&str> = file.path.split('/').collect();
            Self::insert(&mut roots, &parts, idx);
        }
        FileTree { roots }
    }

    fn insert(nodes: &mut Vec<TreeNode>, parts: &[&str], entry_index: usize) {
        if parts.len() == 1 {
            nodes.push(TreeNode {
                name: parts[0].to_string(),
                children: Vec::new(),
                expanded: true,
                entry_index: Some(entry_index),
            });
            return;
        }

        let dir_name = parts[0];
        let dir_pos = nodes.iter().position(|n| n.is_dir() && n.name == dir_name);
        let dir_node = if let Some(pos) = dir_pos {
            &mut nodes[pos]
        } else {
            nodes.push(TreeNode {
                name: dir_name.to_string(),
                children: Vec::new(),
                expanded: true,
                entry_index: None,
            });
            nodes.last_mut().unwrap()
        };

        Self::insert(&mut dir_node.children, &parts[1..], entry_index);
    }

    pub fn toggle_at_visible(&mut self, visible_index: usize) -> bool {
        let mut counter = 0;
        Self::toggle_recursive(&mut self.roots, visible_index, &mut counter)
    }

    fn toggle_recursive(
        nodes: &mut [TreeNode],
        target: usize,
        counter: &mut usize,
    ) -> bool {
        for node in nodes.iter_mut() {
            if *counter == target {
                if node.is_dir() {
                    node.expanded = !node.expanded;
                    return true;
                }
                return false;
            }
            *counter += 1;
            if node.is_dir() && node.expanded {
                if Self::toggle_recursive(&mut node.children, target, counter) {
                    return true;
                }
            }
        }
        false
    }

    pub fn visible_items(&self, files: &[FileEntry]) -> Vec<VisibleItem> {
        let mut items = Vec::new();
        Self::collect_visible(&self.roots, files, 0, &mut items);
        items
    }

    fn collect_visible(
        nodes: &[TreeNode],
        files: &[FileEntry],
        depth: usize,
        items: &mut Vec<VisibleItem>,
    ) {
        for node in nodes {
            if node.is_dir() {
                items.push(VisibleItem {
                    depth,
                    name: node.name.clone(),
                    kind: VisibleItemKind::Directory { expanded: node.expanded },
                });
                if node.expanded {
                    Self::collect_visible(&node.children, files, depth + 1, items);
                }
            } else if let Some(entry_index) = node.entry_index {
                items.push(VisibleItem {
                    depth,
                    name: node.name.clone(),
                    kind: VisibleItemKind::File {
                        status: files[entry_index].status.clone(),
                        entry_index,
                    },
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{FileEntry, FileStatus};

    #[test]
    fn build_empty() {
        let tree = FileTree::build(&[]);
        assert!(tree.roots.is_empty());
    }

    #[test]
    fn build_single_root_file() {
        let files = vec![FileEntry { path: "README.md".into(), status: FileStatus::Added }];
        let tree = FileTree::build(&files);
        assert_eq!(tree.roots.len(), 1);
        assert_eq!(tree.roots[0].name, "README.md");
        assert_eq!(tree.roots[0].entry_index, Some(0));
        assert!(tree.roots[0].children.is_empty());
    }

    #[test]
    fn build_single_file_in_directory() {
        let files = vec![FileEntry { path: "src/main.rs".into(), status: FileStatus::Added }];
        let tree = FileTree::build(&files);
        assert_eq!(tree.roots.len(), 1);
        assert_eq!(tree.roots[0].name, "src");
        assert!(tree.roots[0].entry_index.is_none());
        assert!(tree.roots[0].expanded);
        assert_eq!(tree.roots[0].children.len(), 1);
        assert_eq!(tree.roots[0].children[0].name, "main.rs");
        assert_eq!(tree.roots[0].children[0].entry_index, Some(0));
    }

    #[test]
    fn build_multiple_files_same_directory() {
        let files = vec![
            FileEntry { path: "src/main.rs".into(), status: FileStatus::Added },
            FileEntry { path: "src/lib.rs".into(), status: FileStatus::Modified },
        ];
        let tree = FileTree::build(&files);
        assert_eq!(tree.roots.len(), 1);
        assert_eq!(tree.roots[0].children.len(), 2);
        assert_eq!(tree.roots[0].children[0].name, "main.rs");
        assert_eq!(tree.roots[0].children[1].name, "lib.rs");
    }

    #[test]
    fn build_nested_directories() {
        let files = vec![FileEntry { path: "src/ui/tree.rs".into(), status: FileStatus::Added }];
        let tree = FileTree::build(&files);
        assert_eq!(tree.roots.len(), 1);
        assert_eq!(tree.roots[0].name, "src");
        assert_eq!(tree.roots[0].children.len(), 1);
        assert_eq!(tree.roots[0].children[0].name, "ui");
        assert!(tree.roots[0].children[0].entry_index.is_none());
        assert_eq!(tree.roots[0].children[0].children.len(), 1);
        assert_eq!(tree.roots[0].children[0].children[0].name, "tree.rs");
    }

    #[test]
    fn build_mixed_root_and_nested() {
        let files = vec![
            FileEntry { path: "src/main.rs".into(), status: FileStatus::Added },
            FileEntry { path: "old.rs".into(), status: FileStatus::Deleted },
        ];
        let tree = FileTree::build(&files);
        assert_eq!(tree.roots.len(), 2);
        assert_eq!(tree.roots[0].name, "src");
        assert!(tree.roots[0].entry_index.is_none());
        assert_eq!(tree.roots[1].name, "old.rs");
        assert_eq!(tree.roots[1].entry_index, Some(1));
    }

    #[test]
    fn build_groups_noncontiguous_files() {
        let files = vec![
            FileEntry { path: "src/a.rs".into(), status: FileStatus::Added },
            FileEntry { path: "old.rs".into(), status: FileStatus::Deleted },
            FileEntry { path: "src/b.rs".into(), status: FileStatus::Modified },
        ];
        let tree = FileTree::build(&files);
        assert_eq!(tree.roots.len(), 2);
        assert_eq!(tree.roots[0].name, "src");
        assert_eq!(tree.roots[0].children.len(), 2);
        assert_eq!(tree.roots[0].children[0].name, "a.rs");
        assert_eq!(tree.roots[0].children[0].entry_index, Some(0));
        assert_eq!(tree.roots[0].children[1].name, "b.rs");
        assert_eq!(tree.roots[0].children[1].entry_index, Some(2));
        assert_eq!(tree.roots[1].name, "old.rs");
    }

    #[test]
    fn build_directories_start_expanded() {
        let files = vec![
            FileEntry { path: "src/ui/tree.rs".into(), status: FileStatus::Added },
        ];
        let tree = FileTree::build(&files);
        assert!(tree.roots[0].expanded);
        assert!(tree.roots[0].children[0].expanded);
    }

    #[test]
    fn visible_items_empty_tree() {
        let tree = FileTree::build(&[]);
        let items = tree.visible_items(&[]);
        assert!(items.is_empty());
    }

    #[test]
    fn visible_items_single_root_file() {
        let files = vec![FileEntry { path: "README.md".into(), status: FileStatus::Added }];
        let tree = FileTree::build(&files);
        let items = tree.visible_items(&files);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].depth, 0);
        assert_eq!(items[0].name, "README.md");
        assert_eq!(
            items[0].kind,
            VisibleItemKind::File { status: FileStatus::Added, entry_index: 0 }
        );
    }

    #[test]
    fn visible_items_dir_with_files() {
        let files = vec![
            FileEntry { path: "src/main.rs".into(), status: FileStatus::Added },
            FileEntry { path: "src/lib.rs".into(), status: FileStatus::Modified },
        ];
        let tree = FileTree::build(&files);
        let items = tree.visible_items(&files);
        assert_eq!(items.len(), 3);
        assert_eq!(
            items[0],
            VisibleItem {
                depth: 0,
                name: "src".into(),
                kind: VisibleItemKind::Directory { expanded: true }
            }
        );
        assert_eq!(
            items[1],
            VisibleItem {
                depth: 1,
                name: "main.rs".into(),
                kind: VisibleItemKind::File { status: FileStatus::Added, entry_index: 0 }
            }
        );
        assert_eq!(
            items[2],
            VisibleItem {
                depth: 1,
                name: "lib.rs".into(),
                kind: VisibleItemKind::File { status: FileStatus::Modified, entry_index: 1 }
            }
        );
    }

    #[test]
    fn visible_items_nested_dirs() {
        let files = vec![FileEntry { path: "src/ui/tree.rs".into(), status: FileStatus::Added }];
        let tree = FileTree::build(&files);
        let items = tree.visible_items(&files);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].depth, 0);
        assert_eq!(items[0].name, "src");
        assert_eq!(items[1].depth, 1);
        assert_eq!(items[1].name, "ui");
        assert_eq!(items[2].depth, 2);
        assert_eq!(items[2].name, "tree.rs");
    }

    #[test]
    fn visible_items_collapsed_dir_hides_children() {
        let files = vec![
            FileEntry { path: "src/main.rs".into(), status: FileStatus::Added },
            FileEntry { path: "old.rs".into(), status: FileStatus::Deleted },
        ];
        let mut tree = FileTree::build(&files);
        tree.roots[0].expanded = false;
        let items = tree.visible_items(&files);
        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0],
            VisibleItem {
                depth: 0,
                name: "src".into(),
                kind: VisibleItemKind::Directory { expanded: false }
            }
        );
        assert_eq!(
            items[1],
            VisibleItem {
                depth: 0,
                name: "old.rs".into(),
                kind: VisibleItemKind::File { status: FileStatus::Deleted, entry_index: 1 }
            }
        );
    }

    #[test]
    fn visible_items_nested_collapse_hides_all_descendants() {
        let files = vec![FileEntry { path: "src/ui/tree.rs".into(), status: FileStatus::Added }];
        let mut tree = FileTree::build(&files);
        tree.roots[0].expanded = false;
        let items = tree.visible_items(&files);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "src");
    }

    #[test]
    fn visible_items_inner_collapse() {
        let files = vec![
            FileEntry { path: "src/ui/tree.rs".into(), status: FileStatus::Added },
            FileEntry { path: "src/main.rs".into(), status: FileStatus::Modified },
        ];
        let mut tree = FileTree::build(&files);
        tree.roots[0].children[0].expanded = false; // collapse ui/
        let items = tree.visible_items(&files);
        assert_eq!(items.len(), 3); // src/, ui/ (collapsed), main.rs
        assert_eq!(items[0].name, "src");
        assert_eq!(
            items[1],
            VisibleItem {
                depth: 1,
                name: "ui".into(),
                kind: VisibleItemKind::Directory { expanded: false }
            }
        );
        assert_eq!(items[2].name, "main.rs");
    }

    #[test]
    fn toggle_collapses_expanded_dir() {
        let files = vec![FileEntry { path: "src/main.rs".into(), status: FileStatus::Added }];
        let mut tree = FileTree::build(&files);
        assert!(tree.roots[0].expanded);
        assert!(tree.toggle_at_visible(0));
        assert!(!tree.roots[0].expanded);
    }

    #[test]
    fn toggle_expands_collapsed_dir() {
        let files = vec![FileEntry { path: "src/main.rs".into(), status: FileStatus::Added }];
        let mut tree = FileTree::build(&files);
        tree.roots[0].expanded = false;
        assert!(tree.toggle_at_visible(0));
        assert!(tree.roots[0].expanded);
    }

    #[test]
    fn toggle_file_returns_false() {
        let files = vec![FileEntry { path: "old.rs".into(), status: FileStatus::Added }];
        let mut tree = FileTree::build(&files);
        assert!(!tree.toggle_at_visible(0));
    }

    #[test]
    fn toggle_out_of_bounds() {
        let files = vec![FileEntry { path: "old.rs".into(), status: FileStatus::Added }];
        let mut tree = FileTree::build(&files);
        assert!(!tree.toggle_at_visible(5));
    }

    #[test]
    fn toggle_updates_visible_count() {
        let files = vec![
            FileEntry { path: "src/a.rs".into(), status: FileStatus::Added },
            FileEntry { path: "src/b.rs".into(), status: FileStatus::Modified },
            FileEntry { path: "src/c.rs".into(), status: FileStatus::Deleted },
        ];
        let mut tree = FileTree::build(&files);
        assert_eq!(tree.visible_items(&files).len(), 4);
        tree.toggle_at_visible(0);
        assert_eq!(tree.visible_items(&files).len(), 1);
    }

    #[test]
    fn toggle_nested_dir() {
        let files = vec![
            FileEntry { path: "src/ui/tree.rs".into(), status: FileStatus::Added },
            FileEntry { path: "src/main.rs".into(), status: FileStatus::Modified },
        ];
        let mut tree = FileTree::build(&files);
        // visible: src/(0), ui/(1), tree.rs(2), main.rs(3)
        assert!(tree.toggle_at_visible(1)); // toggle ui/
        assert!(!tree.roots[0].children[0].expanded);
        let items = tree.visible_items(&files);
        assert_eq!(items.len(), 3); // src/, ui/ (collapsed), main.rs
    }
}
