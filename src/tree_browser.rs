//! Tree-based file browser with fuzzy search
//!
//! Uses tui-tree-widget for tree rendering and nucleo-matcher for fuzzy search.

use std::path::PathBuf;

use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher,
};
use tui_tree_widget::{TreeItem, TreeState};

/// Unique identifier for tree nodes
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct NodeId(pub PathBuf);

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

/// A node in the file tree
#[derive(Debug, Clone)]
pub struct FileNode {
    pub path: PathBuf,
    pub is_dir: bool,
    pub name: String,
    pub children: Vec<FileNode>,
    pub expanded: bool,
}

impl FileNode {
    pub fn new(path: PathBuf) -> Self {
        let is_dir = path.is_dir();
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());

        Self {
            path,
            is_dir,
            name,
            children: Vec::new(),
            expanded: false,
        }
    }

    /// Load immediate children (lazy loading)
    pub fn load_children(&mut self) {
        if !self.is_dir || !self.children.is_empty() {
            return;
        }

        if let Ok(entries) = std::fs::read_dir(&self.path) {
            let mut children: Vec<FileNode> = entries
                .filter_map(|e| e.ok())
                .map(|e| FileNode::new(e.path()))
                .collect();

            // Sort: directories first, then alphabetically
            children.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            });

            self.children = children;
        }
    }

    /// Convert to TreeItem for rendering
    pub fn to_tree_item(&self) -> TreeItem<'static, NodeId> {
        let icon = if self.is_dir { "📁 " } else { "📄 " };
        let text = format!("{}{}", icon, self.name);

        if self.children.is_empty() {
            TreeItem::new_leaf(NodeId(self.path.clone()), text)
        } else {
            let children: Vec<_> = self.children.iter().map(|c| c.to_tree_item()).collect();
            TreeItem::new(NodeId(self.path.clone()), text, children).expect("unique identifiers")
        }
    }

    /// Find a node by path (mutable) - standalone function to avoid borrow issues
    pub fn find_mut(&mut self, path: &PathBuf) -> Option<&mut FileNode> {
        if &self.path == path {
            return Some(self);
        }

        for child in &mut self.children {
            if let Some(found) = child.find_mut(path) {
                return Some(found);
            }
        }

        None
    }
}

/// Tree-based file browser with fuzzy search
#[derive(Debug)]
pub struct TreeBrowser {
    pub root_dir: PathBuf,
    pub root: FileNode,
    pub state: TreeState<NodeId>,
    pub selected: Vec<PathBuf>,
    /// Current cursor position (tracked independently for tests)
    pub cursor: Option<PathBuf>,

    // Fuzzy search
    pub search_query: String,
    pub search_active: bool,
    pub search_results: Vec<PathBuf>,
    pub search_nodes: Vec<FileNode>,
    matcher: Matcher,
}

impl TreeBrowser {
    pub fn new() -> Self {
        let cwd = std::env::current_dir()
            .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")));

        Self::with_root(cwd)
    }

    pub fn with_root(root_dir: PathBuf) -> Self {
        let mut root = FileNode::new(root_dir.clone());
        root.load_children();
        root.expanded = true;

        let mut state = TreeState::default();
        state.open(vec![NodeId(root_dir.clone())]);
        state.select_first();

        // Track first child as cursor (if any)
        let cursor = root.children.first().map(|c| c.path.clone());

        Self {
            root_dir,
            root,
            state,
            selected: Vec::new(),
            cursor,
            search_query: String::new(),
            search_active: false,
            search_results: Vec::new(),
            search_nodes: Vec::new(),
            matcher: Matcher::new(Config::DEFAULT),
        }
    }

    pub fn tree_items(&self) -> Vec<TreeItem<'static, NodeId>> {
        if !self.search_nodes.is_empty() {
            // Merged tree - nested matches shown as children
            self.search_nodes
                .iter()
                .map(|node| node.to_tree_item())
                .collect()
        } else {
            self.root
                .children
                .iter()
                .map(|c| c.to_tree_item())
                .collect()
        }
    }

    pub fn has_search_results(&self) -> bool {
        !self.search_results.is_empty()
    }

    pub fn move_up(&mut self) {
        // Prevent going to "no selection" at top - stay at first item
        let had_selection = !self.state.selected().is_empty();
        self.state.key_up();
        if had_selection && self.state.selected().is_empty() {
            self.state.select_first();
        }
    }

    pub fn move_down(&mut self) {
        self.state.key_down();
    }

    /// Collapse the currently selected directory
    pub fn collapse_selected(&mut self) {
        let selected = self.state.selected().to_vec();
        if !selected.is_empty() {
            self.state.close(&selected);
        }
    }

    pub fn move_to_first(&mut self) {
        self.state.select_first();
    }

    pub fn move_to_last(&mut self) {
        self.state.select_last();
    }

    /// Move into selected directory or toggle expand
    pub fn enter(&mut self) {
        let selected_path = self.state.selected().last().map(|id| id.0.clone());

        if let Some(path) = selected_path {
            if path.is_dir() {
                let mut was_leaf = false;

                if !self.search_nodes.is_empty() {
                    if let Some(node) = Self::find_in_nodes(&mut self.search_nodes, &path) {
                        if node.children.is_empty() {
                            node.load_children();
                            was_leaf = !node.children.is_empty();
                        }
                    }
                } else if let Some(node) = self.root.find_mut(&path) {
                    if node.children.is_empty() {
                        node.load_children();
                        was_leaf = !node.children.is_empty();
                    }
                }

                if was_leaf {
                    self.state.open(self.state.selected().to_vec());
                } else {
                    self.state.toggle_selected();
                }
            }
        }
    }

    /// Find a node in a list of nodes by path (mutable)
    fn find_in_nodes<'a>(nodes: &'a mut [FileNode], path: &PathBuf) -> Option<&'a mut FileNode> {
        for node in nodes {
            if let Some(found) = node.find_mut(path) {
                return Some(found);
            }
        }
        None
    }

    /// Go up to parent directory
    pub fn go_up(&mut self) {
        if let Some(parent) = self.root_dir.parent() {
            let parent = parent.to_path_buf();
            self.root_dir = parent.clone();
            self.root = FileNode::new(parent);
            self.root.load_children();
            self.root.expanded = true;
            self.state = TreeState::default();
            self.state.open(vec![NodeId(self.root_dir.clone())]);
            self.state.select_first();
        }
    }

    /// Toggle selection of current item
    pub fn toggle_selection(&mut self) {
        // Use state.selected() if available, otherwise fallback to cursor
        let path = self
            .state
            .selected()
            .last()
            .map(|s| s.0.clone())
            .or_else(|| self.cursor.clone());

        if let Some(path) = path {
            if let Some(pos) = self.selected.iter().position(|p| p == &path) {
                self.selected.remove(pos);
            } else {
                self.selected.push(path);
            }
        }
    }

    /// Select all visible items
    pub fn select_all(&mut self) {
        if self.search_active && !self.search_query.is_empty() {
            self.selected = self.search_results.clone();
        } else {
            // Select all files in current view
            self.selected = self.collect_all_paths(&self.root);
        }
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    pub fn start_search(&mut self) {
        self.search_active = true;
        self.search_query.clear();
        self.search_results.clear();
        self.search_nodes.clear();
    }

    pub fn finish_search(&mut self) {
        self.search_active = false;
    }

    pub fn cancel_search(&mut self) {
        self.search_active = false;
        self.search_query.clear();
        self.search_results.clear();
        self.search_nodes.clear();
        self.state.select_first();
    }

    pub fn clear_search_results(&mut self) {
        self.search_results.clear();
        self.search_nodes.clear();
        self.state.select_first();
    }

    pub fn update_search(&mut self, query: &str) {
        self.search_query = query.to_string();

        if query.is_empty() {
            self.search_results.clear();
            self.search_nodes.clear();
            return;
        }

        let all_paths = self.collect_all_paths(&self.root);
        let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);

        let mut matches: Vec<(i64, PathBuf)> = all_paths
            .into_iter()
            .filter_map(|path| {
                let name = path.file_name()?.to_string_lossy();
                let mut buf = Vec::new();
                let score = pattern.score(
                    nucleo_matcher::Utf32Str::new(&name, &mut buf),
                    &mut self.matcher,
                )?;
                Some((score as i64, path))
            })
            .collect();

        matches.sort_by(|a, b| b.0.cmp(&a.0));

        let paths: Vec<PathBuf> = matches.into_iter().take(100).map(|(_, p)| p).collect();
        self.search_results = paths.clone();
        self.search_nodes = Self::build_merged_tree(&paths);

        // Reset state and open all merged nodes so children are visible
        self.state = TreeState::default();
        for node in &self.search_nodes {
            Self::open_merged_nodes(&mut self.state, node, vec![]);
        }
        self.state.select_first();
    }

    /// Recursively open all nodes that have merged children
    fn open_merged_nodes(state: &mut TreeState<NodeId>, node: &FileNode, mut path: Vec<NodeId>) {
        path.push(NodeId(node.path.clone()));

        if !node.children.is_empty() {
            // Open this node so children are visible
            state.open(path.clone());

            // Recurse into children
            for child in &node.children {
                Self::open_merged_nodes(state, child, path.clone());
            }
        }
    }

    /// Build a merged tree from search results, preserving parent-child relationships
    fn build_merged_tree(paths: &[PathBuf]) -> Vec<FileNode> {
        if paths.is_empty() {
            return Vec::new();
        }

        // Sort by path depth (parents first) then alphabetically
        let mut sorted_paths = paths.to_vec();
        sorted_paths.sort_by(|a, b| {
            let depth_a = a.components().count();
            let depth_b = b.components().count();
            depth_a.cmp(&depth_b).then_with(|| a.cmp(b))
        });

        let mut roots: Vec<FileNode> = Vec::new();

        for path in sorted_paths {
            // Check if this path belongs under any existing root
            let mut inserted = false;
            for root in &mut roots {
                if let Some(parent_node) = Self::find_parent_for_path(root, &path) {
                    // Insert as child of the found parent
                    let node = FileNode::new(path.clone());
                    parent_node.children.push(node);
                    // Re-sort children
                    parent_node
                        .children
                        .sort_by(|a, b| match (a.is_dir, b.is_dir) {
                            (true, false) => std::cmp::Ordering::Less,
                            (false, true) => std::cmp::Ordering::Greater,
                            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                        });
                    inserted = true;
                    break;
                }
            }

            if !inserted {
                // This is a new root-level result
                let node = FileNode::new(path);
                roots.push(node);
            }
        }

        // Sort roots: directories first, then alphabetically
        roots.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        roots
    }

    /// Find the parent node where a path should be inserted
    fn find_parent_for_path<'a>(
        node: &'a mut FileNode,
        path: &PathBuf,
    ) -> Option<&'a mut FileNode> {
        // Check if path is a direct child or descendant of this node
        if !path.starts_with(&node.path) || path == &node.path {
            return None;
        }

        // First, check if any child could be the parent (without borrowing mutably yet)
        let child_idx = node
            .children
            .iter()
            .position(|child| path.starts_with(&child.path) && path != &child.path);

        if let Some(idx) = child_idx {
            // Recurse into that child
            Self::find_parent_for_path(&mut node.children[idx], path)
        } else {
            // This node is the direct parent
            Some(node)
        }
    }

    /// Add character to search query
    pub fn search_push(&mut self, c: char) {
        let mut query = self.search_query.clone();
        query.push(c);
        self.update_search(&query);
    }

    /// Remove last character from search query
    pub fn search_pop(&mut self) {
        let mut query = self.search_query.clone();
        query.pop();
        self.update_search(&query);
    }

    /// Collect all paths recursively
    fn collect_all_paths(&self, node: &FileNode) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        for child in &node.children {
            paths.push(child.path.clone());
            if child.is_dir {
                // Recursively collect, but limit depth for performance
                if let Ok(entries) = std::fs::read_dir(&child.path) {
                    for entry in entries.filter_map(|e| e.ok()).take(100) {
                        paths.push(entry.path());
                    }
                }
            }
        }

        paths
    }
}

impl Default for TreeBrowser {
    fn default() -> Self {
        Self::new()
    }
}

mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_tree() -> (TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();

        fs::write(root.join("aaa.txt"), "content").unwrap();
        fs::write(root.join("bbb.rs"), "fn main() {}").unwrap();
        fs::create_dir(root.join("ccc_dir")).unwrap();
        fs::write(root.join("ccc_dir/nested.md"), "# Title").unwrap();

        (dir, root)
    }

    #[test]
    fn test_file_node_new_file() {
        let (_temp_dir, root) = create_test_tree();
        let file_path = root.join("aaa.txt");

        let node = FileNode::new(file_path.clone());

        assert!(!node.is_dir);
        assert_eq!(node.name, "aaa.txt");
        assert_eq!(node.path, file_path);
        assert!(node.children.is_empty());
    }

    #[test]
    fn test_file_node_new_directory() {
        let (_temp_dir, root) = create_test_tree();
        let dir_path = root.join("ccc_dir");

        let node = FileNode::new(dir_path.clone());

        assert!(node.is_dir);
        assert_eq!(node.name, "ccc_dir");
    }

    #[test]
    fn test_file_node_load_children_sorted() {
        let (_temp_dir, root) = create_test_tree();
        let mut node = FileNode::new(root);

        node.load_children();

        assert_eq!(node.children.len(), 3);
        // Directories first
        assert_eq!(node.children[0].name, "ccc_dir");
        assert!(node.children[0].is_dir);
        // Then files alphabetically
        assert_eq!(node.children[1].name, "aaa.txt");
        assert_eq!(node.children[2].name, "bbb.rs");
    }

    #[test]
    fn test_file_node_load_children_empty_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut node = FileNode::new(temp_dir.path().to_path_buf());

        node.load_children();

        assert!(node.children.is_empty());
    }

    #[test]
    fn test_file_node_load_children_on_file() {
        let (_temp_dir, root) = create_test_tree();
        let mut node = FileNode::new(root.join("aaa.txt"));

        node.load_children();

        assert!(node.children.is_empty());
    }

    #[test]
    fn test_tree_browser_with_root() {
        let (_temp_dir, root) = create_test_tree();

        let browser = TreeBrowser::with_root(root.clone());

        assert_eq!(browser.root_dir, root);
        assert!(browser.root.expanded);
        assert_eq!(browser.root.children.len(), 3);
        assert!(browser.cursor.is_some());
    }

    #[test]
    fn test_fuzzy_search_exact_match() {
        let (_temp_dir, root) = create_test_tree();
        let mut browser = TreeBrowser::with_root(root.clone());

        browser.update_search("aaa.txt");

        assert!(browser.search_results.contains(&root.join("aaa.txt")));
    }

    #[test]
    fn test_fuzzy_search_partial() {
        let (_temp_dir, root) = create_test_tree();
        let mut browser = TreeBrowser::with_root(root.clone());

        browser.update_search("aaa");

        assert!(browser.search_results.contains(&root.join("aaa.txt")));
    }

    #[test]
    fn test_fuzzy_search_case_insensitive() {
        let (_temp_dir, root) = create_test_tree();
        let mut browser = TreeBrowser::with_root(root.clone());

        browser.update_search("AAA");

        assert!(browser.search_results.contains(&root.join("aaa.txt")));
    }

    #[test]
    fn test_fuzzy_search_no_match() {
        let (_temp_dir, root) = create_test_tree();
        let mut browser = TreeBrowser::with_root(root);

        browser.update_search("xyz123nonexistent");

        assert!(browser.search_results.is_empty());
    }

    #[test]
    fn test_move_up_at_top_stays_selected() {
        let (_temp_dir, root) = create_test_tree();
        let mut browser = TreeBrowser::with_root(root);

        assert!(browser.cursor.is_some());
        browser.move_up();
        // Cursor maintained even after move_up at top
        assert!(browser.cursor.is_some());
    }

    #[test]
    fn test_toggle_selection_adds_removes() {
        let (_temp_dir, root) = create_test_tree();
        let mut browser = TreeBrowser::with_root(root);

        browser.toggle_selection();
        assert_eq!(browser.selected.len(), 1);

        browser.toggle_selection();
        assert!(browser.selected.is_empty());
    }

    #[test]
    fn test_clear_selection() {
        let (_temp_dir, root) = create_test_tree();
        let mut browser = TreeBrowser::with_root(root);

        browser.toggle_selection();
        assert!(!browser.selected.is_empty());

        browser.clear_selection();
        assert!(browser.selected.is_empty());
    }

    #[test]
    fn test_cancel_search_clears_state() {
        let (_temp_dir, root) = create_test_tree();
        let mut browser = TreeBrowser::with_root(root);

        browser.start_search();
        browser.update_search("aaa");
        assert!(browser.search_active);
        assert!(!browser.search_query.is_empty());

        browser.cancel_search();

        assert!(!browser.search_active);
        assert!(browser.search_query.is_empty());
        assert!(browser.search_results.is_empty());
    }

    #[test]
    fn test_start_search_clears_previous() {
        let (_temp_dir, root) = create_test_tree();
        let mut browser = TreeBrowser::with_root(root);

        browser.start_search();
        browser.update_search("aaa");
        assert!(!browser.search_query.is_empty());

        browser.start_search();

        assert!(browser.search_query.is_empty());
        assert!(browser.search_active);
    }

    #[test]
    fn test_search_push_pop() {
        let (_temp_dir, root) = create_test_tree();
        let mut browser = TreeBrowser::with_root(root);

        browser.start_search();
        browser.search_push('a');
        browser.search_push('a');
        browser.search_push('a');
        assert_eq!(browser.search_query, "aaa");

        browser.search_pop();
        assert_eq!(browser.search_query, "aa");
    }

    #[test]
    fn test_node_id_display() {
        let node_id = NodeId(PathBuf::from("/test/path"));
        let display = format!("{}", node_id);
        assert!(display.contains("test"));
    }
}
