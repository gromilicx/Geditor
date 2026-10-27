use anyhow::{Context, Result, anyhow};
use ratatui::style::{Color, Style};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tui_textarea::TextArea;

pub const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024;

pub struct Document {
    pub path: Option<PathBuf>,
    pub title: String,
    pub textarea: TextArea<'static>,
    pub original: String,
    pub eol: &'static str,
    pub highlight_dirty: bool,
}

impl Document {
    pub fn empty(title: &str) -> Self {
        Self::create(None, title.to_string(), String::new(), "\n")
    }

    pub fn load(path: &Path) -> Result<Self> {
        let meta =
            fs::metadata(path).with_context(|| format!("Cannot inspect {}", path.display()))?;
        if !meta.is_file() {
            return Err(anyhow!("Not a regular file"));
        }
        if meta.len() > MAX_FILE_SIZE {
            return Err(anyhow!("File exceeds the 2 MiB editor limit"));
        }
        let content = fs::read_to_string(path)
            .with_context(|| format!("Not UTF-8 text: {}", path.display()))?;
        if content.contains('\0') {
            return Err(anyhow!("Binary data is not supported"));
        }
        let eol = if content.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let normalized = content.replace("\r\n", "\n");
        if normalized.bytes().filter(|&b| b == b'\n').count() >= u16::MAX as usize {
            return Err(anyhow!("This buffer exceeds the 65,535 line limit"));
        }
        let title = path
            .file_name()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(Self::create(
            Some(path.to_path_buf()),
            title,
            normalized,
            eol,
        ))
    }

    fn create(path: Option<PathBuf>, title: String, content: String, eol: &'static str) -> Self {
        let mut textarea = TextArea::from(content.split('\n').map(str::to_owned));
        textarea.set_line_number_style(Style::default().fg(Color::Rgb(101, 116, 143)));
        textarea.set_cursor_line_style(Style::default().bg(Color::Rgb(37, 44, 62)));
        textarea.set_cursor_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(133, 205, 255)),
        );
        textarea.set_selection_style(Style::default().bg(Color::Rgb(61, 88, 127)));
        textarea.set_style(
            Style::default()
                .fg(Color::Rgb(210, 223, 238))
                .bg(Color::Rgb(23, 28, 40)),
        );
        textarea.set_search_style(Style::default().bg(Color::Rgb(115, 83, 42)));
        textarea.set_undo_coalescing(true);
        textarea.set_max_histories(400);
        Self {
            path,
            title,
            textarea,
            original: content,
            eol,
            highlight_dirty: true,
        }
    }

    pub fn is_markdown(&self) -> bool {
        let path = self
            .path
            .as_deref()
            .unwrap_or_else(|| Path::new(&self.title));
        path.extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| {
                ["md", "markdown", "mdown", "mkd", "mkdn"]
                    .contains(&ext.to_ascii_lowercase().as_str())
            })
    }

    pub fn text(&self) -> String {
        self.textarea.lines().join("\n")
    }
    pub fn is_dirty(&self) -> bool {
        !self
            .textarea
            .lines()
            .iter()
            .map(String::as_str)
            .eq(self.original.split('\n'))
    }

    pub fn save(&mut self) -> Result<PathBuf> {
        let path = self
            .path
            .clone()
            .ok_or_else(|| anyhow!("Untitled file: use Ctrl+Shift+S / Save As"))?;
        let text = self.text();
        let encoded = if self.eol == "\r\n" {
            text.replace('\n', "\r\n")
        } else {
            text.clone()
        };
        fs::write(&path, encoded).with_context(|| format!("Cannot save {}", path.display()))?;
        self.original = text;
        Ok(path)
    }

    pub fn save_as(&mut self, path: PathBuf) -> Result<PathBuf> {
        if path.exists() {
            return Err(anyhow!("File already exists; refusing to overwrite"));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = self.text();
        let encoded = if self.eol == "\r\n" {
            text.replace('\n', "\r\n")
        } else {
            text.clone()
        };
        // Create exclusively so an existing file (including a symlink) cannot be overwritten.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(encoded.as_bytes())?;
        self.original = text;
        self.title = path
            .file_name()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.path = Some(path.clone());
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn initial_buffer_is_clean() {
        let mut doc = Document::empty("untitled");
        assert!(!doc.is_dirty());
        doc.textarea.insert_str("fn main() {}");
        assert!(doc.is_dirty());
    }
}

#[cfg(test)]
mod io_tests {
    use super::*;

    #[test]
    fn save_as_preserves_crlf_and_refuses_existing_files() {
        let root = std::env::temp_dir().join(format!("geditor-save-as-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.txt");
        let copy = root.join("copy.txt");
        fs::write(&source, "one\r\ntwo\r\n").unwrap();
        let mut doc = Document::load(&source).unwrap();
        doc.textarea.insert_str("added ");
        doc.save_as(copy.clone()).unwrap();
        assert_eq!(fs::read(&copy).unwrap(), b"added one\r\ntwo\r\n");
        assert!(!doc.is_dirty());
        doc.textarea.insert_str("unsaved ");
        assert!(doc.save_as(source.clone()).is_err());
        assert_eq!(fs::read(&source).unwrap(), b"one\r\ntwo\r\n");
        assert!(doc.is_dirty());
        assert_eq!(doc.path.as_ref(), Some(&copy));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_binary_invalid_utf8_and_oversized_files() {
        let root = std::env::temp_dir().join(format!("geditor-invalid-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let file = root.join("input");
        fs::write(&file, b"text\0binary").unwrap();
        assert!(Document::load(&file).is_err());
        fs::write(&file, [0xff, 0xfe]).unwrap();
        assert!(Document::load(&file).is_err());
        fs::File::create(&file)
            .unwrap()
            .set_len(MAX_FILE_SIZE + 1)
            .unwrap();
        assert!(Document::load(&file).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preserves_crlf_on_save() {
        let root = std::env::temp_dir().join(format!("geditor-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&root);
        let file = root.join("crlf.txt");
        fs::write(&file, "one\r\ntwo\r\n").unwrap();
        let mut doc = Document::load(&file).unwrap();
        assert_eq!(doc.text(), "one\ntwo\n");
        doc.textarea.insert_str("added");
        doc.save().unwrap();
        assert!(fs::read_to_string(&file).unwrap().contains("\r\n"));
        let _ = fs::remove_file(&file);
        let _ = fs::remove_dir(&root);
    }
}
