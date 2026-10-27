use std::{collections::HashSet, fs, path::PathBuf};

#[derive(Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub depth: usize,
    pub is_dir: bool,
}

pub struct FileTree {
    root: PathBuf,
    expanded: HashSet<PathBuf>,
    pub entries: Vec<Entry>,
    pub selected: usize,
    pub scroll: usize,
    pub show_hidden: bool,
}

impl FileTree {
    pub fn new(root: PathBuf) -> Self {
        let mut expanded = HashSet::new();
        expanded.insert(root.clone());
        let mut tree = Self {
            root,
            expanded,
            entries: Vec::new(),
            selected: 0,
            scroll: 0,
            show_hidden: false,
        };
        tree.refresh();
        tree
    }

    pub fn refresh(&mut self) {
        let selected_path = self.entries.get(self.selected).map(|e| e.path.clone());
        self.entries.clear();
        self.entries.push(Entry {
            path: self.root.clone(),
            depth: 0,
            is_dir: true,
        });
        self.visit(self.root.clone(), 1, 0);
        self.selected = selected_path
            .and_then(|p| self.entries.iter().position(|e| e.path == p))
            .unwrap_or_else(|| self.selected.min(self.entries.len().saturating_sub(1)));
    }

    fn visit(&mut self, path: PathBuf, depth: usize, counter: usize) {
        if depth > 20
            || counter > 6_000
            || self.entries.len() >= 6_000
            || !self.expanded.contains(&path)
        {
            return;
        }
        let Ok(iter) = fs::read_dir(&path) else {
            return;
        };
        let mut children = iter
            .flatten()
            .filter_map(|d| {
                let typ = d.file_type().ok()?;
                let p = d.path();
                let name = d.file_name().to_string_lossy().to_string();
                if !self.show_hidden && name.starts_with('.') {
                    return None;
                }
                if typ.is_dir()
                    && [
                        "node_modules",
                        "target",
                        "dist",
                        "build",
                        ".next",
                        ".git",
                        ".idea",
                        "vendor",
                        ".venv",
                    ]
                    .contains(&name.as_str())
                {
                    return None;
                }
                // Do not follow symlinked directories (prevents recursive loops).
                Some((p, typ.is_dir()))
            })
            .collect::<Vec<_>>();
        children.sort_by(|a, b| {
            b.1.cmp(&a.1).then_with(|| {
                a.0.file_name()
                    .map(|s| s.to_string_lossy().to_lowercase())
                    .unwrap_or_default()
                    .cmp(
                        &b.0.file_name()
                            .map(|s| s.to_string_lossy().to_lowercase())
                            .unwrap_or_default(),
                    )
            })
        });
        for (child, is_dir) in children {
            if self.entries.len() >= 6_000 {
                break;
            }
            self.entries.push(Entry {
                path: child.clone(),
                depth,
                is_dir,
            });
            if is_dir {
                self.visit(child, depth + 1, self.entries.len());
            }
        }
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        self.entries.get(self.selected)
    }

    pub fn toggle_or_open(&mut self) -> Option<PathBuf> {
        let entry = self.selected_entry()?.clone();
        if entry.is_dir {
            if !self.expanded.insert(entry.path.clone()) {
                self.expanded.remove(&entry.path);
            }
            self.refresh();
            None
        } else {
            Some(entry.path)
        }
    }

    pub fn collapse_or_parent(&mut self) {
        let Some(entry) = self.selected_entry().cloned() else {
            return;
        };
        if entry.is_dir && self.expanded.contains(&entry.path) && entry.path != self.root {
            self.expanded.remove(&entry.path);
            self.refresh();
        } else if let Some(parent) = entry.path.parent()
            && let Some(index) = self.entries.iter().position(|e| e.path == parent)
        {
            self.selected = index;
        }
    }

    pub fn expand_or_open(&mut self) -> Option<PathBuf> {
        let entry = self.selected_entry().cloned()?;
        if entry.is_dir {
            if !self.expanded.contains(&entry.path) {
                self.expanded.insert(entry.path.clone());
                self.refresh();
            } else if self.selected + 1 < self.entries.len() {
                self.selected += 1;
            }
            None
        } else {
            Some(entry.path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn root_is_always_visible() {
        let cwd = std::env::current_dir().unwrap();
        let tree = FileTree::new(cwd.clone());
        assert_eq!(tree.entries[0].path, cwd);
    }
}
