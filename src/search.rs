use crate::document::MAX_FILE_SIZE;
use globset::{Glob, GlobSetBuilder};
use ignore::WalkBuilder;
use regex::{Regex, RegexBuilder};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Default)]
pub struct Options {
    pub case: bool,
    pub regex: bool,
    pub word: bool,
    pub hidden: bool,
    pub ignored: bool,
    pub glob: String,
}
#[derive(Clone, Debug)]
pub struct Hit {
    pub path: PathBuf,
    pub row: usize,
    pub col: usize,
    pub end: usize,
    pub preview: String,
}
#[derive(Default)]
pub struct Results {
    pub hits: Vec<Hit>,
    pub files: Vec<PathBuf>,
    pub scanned: usize,
    pub skipped: usize,
    pub truncated: bool,
    pub error: Option<String>,
}
pub struct Job {
    pub rx: Receiver<Results>,
    cancel: Arc<AtomicBool>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
pub fn compile(query: &str, options: &Options) -> Result<Regex, String> {
    let text = if options.regex {
        query.to_owned()
    } else {
        regex::escape(query)
    };
    let text = if options.word {
        format!(r"\b(?:{text})\b")
    } else {
        text
    };
    RegexBuilder::new(&text)
        .case_insensitive(!options.case)
        .size_limit(2 * 1024 * 1024)
        .build()
        .map_err(|e| e.to_string().replace('\n', " "))
}
pub fn in_text(path: &Path, text: &str, re: &Regex, limit: usize) -> Vec<Hit> {
    let mut hits = Vec::new();
    for (row, raw) in text.split('\n').enumerate() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let mut last_byte = 0;
        let mut column = 0;
        for found in re.find_iter(line) {
            column += line[last_byte..found.start()].chars().count();
            last_byte = found.start();
            let col = column;
            let end = col + found.as_str().chars().count();
            // Center previews on the match, even when a line is thousands of columns long.
            let start = line[..found.start()]
                .char_indices()
                .rev()
                .nth(39)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let snippet: String = line[start..].chars().take(160).collect();
            hits.push(Hit {
                path: path.to_owned(),
                row,
                col,
                end,
                preview: format!(
                    "{}{}",
                    if start > 0 { "..." } else { "" },
                    snippet.replace('\t', "  ")
                ),
            });
            if hits.len() >= limit {
                return hits;
            }
        }
    }
    hits
}
pub fn start(
    root: PathBuf,
    query: Option<String>,
    options: Options,
    open: HashMap<PathBuf, String>,
) -> Job {
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let cancelled = cancel.clone();
    thread::spawn(move || {
        let result = scan(&root, query.as_deref(), &options, &open, &cancelled);
        let _ = tx.send(result);
    });
    Job { rx, cancel }
}
fn scan(
    root: &Path,
    query: Option<&str>,
    options: &Options,
    open: &HashMap<PathBuf, String>,
    cancel: &AtomicBool,
) -> Results {
    let mut result = Results::default();
    let re = match query.map(|q| compile(q, options)).transpose() {
        Ok(r) => r,
        Err(e) => {
            result.error = Some(e);
            return result;
        }
    };
    let mut gb = GlobSetBuilder::new();
    for pattern in options
        .glob
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        match Glob::new(pattern) {
            Ok(g) => {
                gb.add(g);
            }
            Err(e) => {
                result.error = Some(e.to_string());
                return result;
            }
        }
    }
    let globs = gb.build().unwrap();
    let mut walker = WalkBuilder::new(root);
    walker
        .hidden(!options.hidden)
        .git_ignore(!options.ignored)
        .git_exclude(!options.ignored)
        .git_global(!options.ignored)
        .require_git(false)
        .follow_links(false)
        .max_depth(Some(40));
    let include_ignored = options.ignored;
    walker.filter_entry(move |e| {
        if e.depth() == 0 || !e.file_type().is_some_and(|f| f.is_dir()) {
            return true;
        }
        let name = e.file_name().to_string_lossy();
        name != ".git"
            && (include_ignored
                || !["target", "node_modules", "vendor", ".venv", "dist", "build"]
                    .contains(&name.as_ref()))
    });
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut total = 0usize;
    let mut seen = HashSet::new();
    for (visited, entry) in walker.build().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return result;
        }
        if visited >= 100_000 || result.scanned >= 20_000 || Instant::now() > deadline {
            result.truncated = true;
            break;
        }
        let entry = match entry {
            Ok(e) => e,
            Err(_) => {
                result.skipped += 1;
                continue;
            }
        };
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap_or(path);
        if !globs.is_empty()
            && !globs.is_match(rel)
            && !globs.is_match(path.file_name().unwrap_or_default())
        {
            continue;
        }
        seen.insert(path.to_owned());
        result.scanned += 1;
        if re.is_none() {
            result.files.push(path.to_owned());
            continue;
        }
        let text = if let Some(text) = open.get(path) {
            text.clone()
        } else {
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => {
                    result.skipped += 1;
                    continue;
                }
            };
            if meta.len() > MAX_FILE_SIZE {
                result.skipped += 1;
                continue;
            }
            let mut text = String::new();
            let read = fs::File::open(path)
                .and_then(|f| f.take(MAX_FILE_SIZE + 1).read_to_string(&mut text));
            if read.is_err() || text.len() as u64 > MAX_FILE_SIZE || text.contains('\0') {
                result.skipped += 1;
                continue;
            }
            text
        };
        total += text.len();
        if total > 64 * 1024 * 1024 {
            result.truncated = true;
            break;
        }
        if let Some(re) = &re {
            result.hits.extend(in_text(
                path,
                &text,
                re,
                5001usize.saturating_sub(result.hits.len()),
            ));
        }
        if result.hits.len() > 5000 {
            result.hits.truncate(5000);
            result.truncated = true;
            break;
        }
    }
    // Open unsaved buffers may not have a disk entry yet.
    if let Some(re) = &re {
        for (path, text) in open {
            if !seen.contains(path)
                && path.starts_with(root)
                && (globs.is_empty() || globs.is_match(path.strip_prefix(root).unwrap_or(path)))
            {
                result.hits.extend(in_text(
                    path,
                    text,
                    re,
                    5001usize.saturating_sub(result.hits.len()),
                ));
                if result.hits.len() > 5000 {
                    result.hits.truncate(5000);
                    result.truncated = true;
                    break;
                }
            }
        }
        result.hits.sort_by(|a, b| {
            a.path
                .cmp(&b.path)
                .then(a.row.cmp(&b.row))
                .then(a.col.cmp(&b.col))
        });
    } else {
        result.files.sort();
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_columns_word_case_and_regex() {
        let opt = Options {
            word: true,
            ..Options::default()
        };
        let re = compile("čar", &opt).unwrap();
        let hits = in_text(Path::new("x"), "ž čar ČAR čarobno", &re, 10);
        assert_eq!(hits.len(), 2);
        assert_eq!((hits[0].row, hits[0].col, hits[0].end), (0, 2, 5));
        assert!(
            compile(
                "[",
                &Options {
                    regex: true,
                    ..Options::default()
                }
            )
            .is_err()
        );
    }
    #[test]
    fn workspace_honors_filters_and_unsaved_buffers() {
        let root = std::env::temp_dir().join(format!("geditor-search-{}", std::process::id()));
        fs::create_dir_all(root.join("target")).unwrap();
        fs::write(root.join(".gitignore"), "ignored.rs\n").unwrap();
        fs::write(root.join("a.rs"), "needle disk").unwrap();
        fs::write(root.join("ignored.rs"), "needle").unwrap();
        fs::write(root.join("target/a.rs"), "needle").unwrap();
        let mut open = HashMap::new();
        open.insert(root.join("a.rs"), "other\nneedle ž".into());
        let result = scan(
            &root,
            Some("needle"),
            &Options {
                glob: "*.rs".into(),
                ..Options::default()
            },
            &open,
            &AtomicBool::new(false),
        );
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].row, 1);
        fs::remove_dir_all(root).unwrap();
    }
}
