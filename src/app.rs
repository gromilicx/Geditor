use crate::{
    ai::{self, ChatMessage},
    config::Settings,
    document::Document,
    highlight::Highlighter,
    markdown::{Cache as MarkdownCache, Mode as MarkdownMode},
    search::{self, Hit, Job, Options, Results},
    shell::Shell,
    tree::FileTree,
};
use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::{Position, Rect};
use std::{collections::HashMap, env, fs, path::PathBuf, sync::mpsc::Receiver};
use tui_textarea::CursorMove;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Tree,
    Editor,
    Chat,
    Terminal,
}
impl Focus {
    pub fn name(self) -> &'static str {
        match self {
            Self::Tree => "EXPLORER",
            Self::Editor => "EDITOR",
            Self::Chat => "AI CHAT",
            Self::Terminal => "TERMINAL",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Popup {
    Files,
    Find,
    Project,
    SaveAs,
    Open,
    Menu,
    Settings,
    Help,
    About,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Theme,
    ColorMode,
    Accent,
    Background,
    Foreground,
    Ascii,
    Clock,
    TabWidth,
    LineNumbers,
    Font,
    Tree,
    Editor,
    Chat,
    Terminal,
    Minimap,
    Markdown,
    TreeWidth,
    ChatWidth,
    TermHeight,
    MapWidth,
    Model,
    CustomModel,
    FetchModels,
    Profile,
    ProfileName,
    ApiKey,
    BaseUrl,
    Context,
    Author,
}
pub const APPEARANCE: &[Field] = &[
    Field::Theme,
    Field::ColorMode,
    Field::Accent,
    Field::Background,
    Field::Foreground,
    Field::Ascii,
    Field::Clock,
    Field::TabWidth,
    Field::LineNumbers,
    Field::Font,
];
pub const PANELS: &[Field] = &[
    Field::Tree,
    Field::Editor,
    Field::Chat,
    Field::Terminal,
    Field::Minimap,
    Field::Markdown,
    Field::TreeWidth,
    Field::ChatWidth,
    Field::TermHeight,
    Field::MapWidth,
];
pub const AI: &[Field] = &[
    Field::Profile,
    Field::ProfileName,
    Field::ApiKey,
    Field::Model,
    Field::CustomModel,
    Field::FetchModels,
    Field::BaseUrl,
    Field::Context,
];
pub const GENERAL: &[Field] = &[Field::Author];
pub struct Dropdown {
    pub field: Field,
    pub items: Vec<String>,
    pub selected: usize,
}
#[derive(Clone, Copy)]
pub enum Command {
    New,
    Open,
    Files,
    Save,
    SaveAs,
    Close,
    Quit,
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
    SelectAll,
    Find,
    Project,
    NextMatch,
    PrevMatch,
    NextTab,
    PrevTab,
    Toggle(Focus),
    Map,
    MarkdownToggle,
    MarkdownFull,
    MarkdownFocus,
    Zoom,
    Restore,
    Settings,
    Help,
    About,
    Refresh,
}
pub struct App {
    pub root: PathBuf,
    pub tree: FileTree,
    pub docs: Vec<Document>,
    pub active: usize,
    pub has_document: bool,
    pub highlighter: Highlighter,
    pub settings: Settings,
    pub settings_path: PathBuf,
    pub settings_original: Option<Settings>,
    pub settings_tab: usize,
    pub settings_row: usize,
    pub dropdown: Option<Dropdown>,
    pub editing: bool,
    pub edit_text: String,
    pub focus: Focus,
    pub show_tree: bool,
    pub show_editor: bool,
    pub show_chat: bool,
    pub show_terminal: bool,
    pub show_minimap: bool,
    pub zoom: Option<Focus>,
    pub shell: Option<Shell>,
    pub shell_error: Option<String>,
    pub chat: Vec<ChatMessage>,
    pub chat_input: String,
    pub clipboard: String,
    pub chat_rx: Option<Receiver<Result<String, String>>>,
    pub models_rx: Option<Receiver<Result<Vec<String>, String>>>,
    pub chat_scroll: usize,
    pub popup: Option<Popup>,
    pub popup_input: String,
    pub input_cursor: usize,
    pub glob_cursor: usize,
    pub search_buffer: Option<usize>,
    pub file_index: Vec<PathBuf>,
    pub file_hits: Vec<PathBuf>,
    pub selected: usize,
    pub index_job: Option<Job>,
    pub search_job: Option<Job>,
    pub results: Results,
    pub search_options: Options,
    pub search_pending: bool,
    pub search_glob_edit: bool,
    pub last_query: String,
    pub menu: usize,
    pub menu_row: usize,
    pub menu_rects: Vec<Rect>,
    pub popup_rect: Rect,
    pub dropdown_rect: Rect,
    pub result_rect: Rect,
    pub settings_rects: Vec<Rect>,
    pub tab_rects: Vec<Rect>,
    pub tree_rect: Rect,
    pub editor_rect: Rect,
    pub chat_rect: Rect,
    pub term_rect: Rect,
    pub map_rect: Rect,
    pub markdown_rect: Rect,
    pub markdown_mode: MarkdownMode,
    pub markdown_previous_zoom: Option<Option<Focus>>,
    pub markdown: MarkdownCache,
    pub status: String,
    pub prefix: bool,
    pub quit_armed: bool,
    pub running: bool,
}
impl App {
    pub fn new(root: PathBuf, settings: Settings, settings_path: PathBuf) -> Self {
        let mut doc = Document::empty("untitled");
        settings.style_doc(&mut doc);
        let (tree, editor, chat, terminal, map) = (
            settings.tree,
            settings.editor,
            settings.chat,
            settings.terminal,
            settings.minimap,
        );
        let markdown_mode = if settings.markdown_preview {
            MarkdownMode::Split
        } else {
            MarkdownMode::Source
        };
        let mut app = Self {
            tree: FileTree::new(root.clone()),
            root,
            docs: vec![doc],
            active: 0,
            has_document: false,
            highlighter: Highlighter::new(),
            settings,
            settings_path,
            settings_original: None,
            settings_tab: 0,
            settings_row: 0,
            dropdown: None,
            editing: false,
            edit_text: String::new(),
            focus: Focus::Editor,
            show_tree: tree,
            show_editor: editor,
            show_chat: chat,
            show_terminal: terminal,
            show_minimap: map,
            zoom: None,
            shell: None,
            shell_error: None,
            chat: vec![ChatMessage { role: "assistant", content: "Welcome to GEditor. Configure your model and key in Settings. File context is sent only when you send a message and Include file context is enabled.".into() }],
            chat_input: String::new(),
            clipboard: String::new(),
            chat_rx: None,
            models_rx: None,
            chat_scroll: 0,
            popup: None,
            popup_input: String::new(),
            input_cursor: 0,
            glob_cursor: 0,
            search_buffer: None,
            file_index: Vec::new(),
            file_hits: Vec::new(),
            selected: 0,
            index_job: None,
            search_job: None,
            results: Results::default(),
            search_options: Options::default(),
            search_pending: false,
            search_glob_edit: false,
            last_query: String::new(),
            menu: 0,
            menu_row: 0,
            menu_rects: Vec::new(),
            popup_rect: Rect::default(),
            dropdown_rect: Rect::default(),
            result_rect: Rect::default(),
            settings_rects: Vec::new(),
            tab_rects: Vec::new(),
            tree_rect: Rect::default(),
            editor_rect: Rect::default(),
            chat_rect: Rect::default(),
            term_rect: Rect::default(),
            map_rect: Rect::default(),
            markdown_rect: Rect::default(),
            markdown_mode,
            markdown_previous_zoom: None,
            markdown: MarkdownCache::default(),
            status: "READY  |  Ctrl+E menu  |  Ctrl+P files  |  Ctrl+Tab panels".into(),
            prefix: false,
            quit_armed: false,
            running: true,
        };
        app.normalize_focus();
        app
    }
    fn set_markdown_mode(&mut self, mode: MarkdownMode) {
        if mode == MarkdownMode::Preview && self.markdown_mode != MarkdownMode::Preview {
            self.markdown_previous_zoom = Some(self.zoom);
            self.zoom = Some(Focus::Editor);
        } else if mode != MarkdownMode::Preview && self.markdown_mode == MarkdownMode::Preview {
            self.zoom = self.markdown_previous_zoom.take().unwrap_or(self.zoom);
        }
        self.markdown_mode = mode;
    }
    pub fn doc(&self) -> &Document {
        &self.docs[self.active]
    }
    pub fn doc_mut(&mut self) -> &mut Document {
        &mut self.docs[self.active]
    }
    pub fn enabled(&self, p: Focus) -> bool {
        match p {
            Focus::Tree => self.show_tree,
            Focus::Editor => self.show_editor,
            Focus::Chat => self.show_chat,
            Focus::Terminal => self.show_terminal,
        }
    }
    pub fn enabled_panels(&self) -> Vec<Focus> {
        [Focus::Tree, Focus::Editor, Focus::Chat, Focus::Terminal]
            .into_iter()
            .filter(|p| self.enabled(*p))
            .collect()
    }
    pub fn normalize_focus(&mut self) {
        if !self.enabled(self.focus) {
            self.focus = self
                .enabled_panels()
                .first()
                .copied()
                .unwrap_or(Focus::Editor);
        }
        if self.zoom.is_some() {
            self.zoom = Some(self.focus);
        }
    }
    pub fn cycle(&mut self, back: bool) {
        let list = self.enabled_panels();
        if list.is_empty() {
            self.show_editor = true;
            self.focus = Focus::Editor;
            return;
        }
        let i = list.iter().position(|p| *p == self.focus).unwrap_or(0);
        let n = if back {
            (i + list.len() - 1) % list.len()
        } else {
            (i + 1) % list.len()
        };
        self.focus = list[n];
        if self.zoom.is_some() {
            self.zoom = Some(self.focus);
        }
        if self.focus == Focus::Terminal {
            self.ensure_shell();
        }
    }
    pub fn toggle(&mut self, p: Focus) {
        match p {
            Focus::Tree => self.show_tree = !self.show_tree,
            Focus::Editor => self.show_editor = !self.show_editor,
            Focus::Chat => self.show_chat = !self.show_chat,
            Focus::Terminal => self.show_terminal = !self.show_terminal,
        }
        if self.enabled(p) {
            self.focus = p;
        }
        self.normalize_focus();
    }
    pub fn reveal(&mut self, p: Focus) {
        if !self.enabled(p) {
            self.toggle(p);
        }
        self.focus = p;
        if self.zoom.is_some() {
            self.zoom = Some(p);
        }
    }
    pub fn open_file(&mut self, path: PathBuf) {
        self.markdown.focused = false;
        if self.markdown_mode == MarkdownMode::Preview {
            self.set_markdown_mode(MarkdownMode::Split);
        }
        let path = fs::canonicalize(&path).unwrap_or(path);
        if let Some(i) = self
            .docs
            .iter()
            .position(|d| d.path.as_ref() == Some(&path))
        {
            self.active = i;
            self.has_document = true;
            self.reveal(Focus::Editor);
            return;
        }
        let loaded = if path.exists() {
            Document::load(&path)
        } else {
            let mut d = Document::empty(&path.file_name().unwrap_or_default().to_string_lossy());
            d.path = Some(path.clone());
            Ok(d)
        };
        match loaded {
            Ok(mut d) => {
                self.settings.style_doc(&mut d);
                if !self.has_document {
                    self.docs[0] = d;
                    self.active = 0;
                } else {
                    self.docs.push(d);
                    self.active = self.docs.len() - 1;
                }
                self.has_document = true;
                self.reveal(Focus::Editor);
                self.status = format!("OPEN  {}", path.display());
            }
            Err(e) => self.status = format!("OPEN FAILED  {e:#}"),
        }
    }
    pub fn new_file(&mut self) {
        self.markdown.focused = false;
        let mut doc = Document::empty("untitled");
        self.settings.style_doc(&mut doc);
        if self.has_document {
            self.docs.push(doc);
            self.active = self.docs.len() - 1;
        } else {
            self.docs[0] = doc;
            self.active = 0;
        }
        self.has_document = true;
        self.reveal(Focus::Editor);
        self.status = "NEW BUFFER  Ctrl+S to save".into();
    }
    pub fn save(&mut self) {
        if !self.has_document {
            return;
        }
        if self.doc().path.is_none() {
            self.open_popup(Popup::SaveAs);
            return;
        }
        match self.doc_mut().save() {
            Ok(p) => {
                self.status = format!("SAVED  {}", p.display());
                self.tree.refresh();
            }
            Err(e) => self.status = format!("SAVE FAILED  {e:#}"),
        }
    }
    pub fn close_file(&mut self) {
        if !self.has_document {
            return;
        }
        if self.doc().is_dirty() {
            self.status = "Unsaved changes: save before closing this file".into();
            return;
        }
        self.markdown.focused = false;
        if self.markdown_mode == MarkdownMode::Preview {
            self.set_markdown_mode(MarkdownMode::Split);
        }
        self.docs.remove(self.active);
        self.results = Results::default();
        self.search_buffer = None;
        if self.docs.is_empty() {
            let mut doc = Document::empty("untitled");
            self.settings.style_doc(&mut doc);
            self.docs.push(doc);
            self.has_document = false;
        }
        self.active = self.active.min(self.docs.len() - 1);
    }
    pub fn next_tab(&mut self, back: bool) {
        self.markdown.focused = false;
        if self.markdown_mode == MarkdownMode::Preview {
            self.set_markdown_mode(MarkdownMode::Split);
        }
        if self.has_document {
            self.active = if back {
                (self.active + self.docs.len() - 1) % self.docs.len()
            } else {
                (self.active + 1) % self.docs.len()
            };
            self.reveal(Focus::Editor);
        }
    }
    pub fn quit(&mut self) {
        if self.docs.iter().any(Document::is_dirty) && !self.quit_armed {
            self.quit_armed = true;
            self.status = "UNSAVED CHANGES  Ctrl+Q again to discard and quit".into();
        } else {
            self.running = false;
        }
    }
    pub fn open_popup(&mut self, p: Popup) {
        self.popup = Some(p);
        self.selected = 0;
        self.popup_input.clear();
        self.input_cursor = 0;
        self.glob_cursor = self.search_options.glob.len();
        self.prefix = false;
        match p {
            Popup::Files => {
                self.file_hits.clear();
                self.index_job = Some(search::start(
                    self.root.clone(),
                    None,
                    Options::default(),
                    HashMap::new(),
                ));
                self.status = "Indexing files in background...".into();
            }
            Popup::Find | Popup::Project => {
                self.popup_input = self.last_query.clone();
                self.input_cursor = self.popup_input.len();
                self.search_pending = true;
                self.search_glob_edit = false;
                self.results = Results::default();
                self.search_job = None;
                if p == Popup::Find {
                    self.run_search();
                }
            }
            Popup::Settings => {
                self.settings_original = Some(self.settings.clone());
                self.settings_row = 0;
                self.editing = false;
                self.dropdown = None;
            }
            Popup::Menu => self.menu_row = 0,
            _ => {}
        }
    }
    pub fn filter_files(&mut self) {
        let q = self.popup_input.to_lowercase();
        let mut files: Vec<_> = self
            .file_index
            .iter()
            .filter(|p| {
                p.strip_prefix(&self.root)
                    .unwrap_or(p)
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&q)
            })
            .cloned()
            .collect();
        files.sort_by_key(|p| {
            let rel = p
                .strip_prefix(&self.root)
                .unwrap_or(p)
                .to_string_lossy()
                .to_lowercase();
            (!rel.starts_with(&q), rel.len(), rel)
        });
        files.truncate(300);
        self.file_hits = files;
        self.selected = 0;
    }
    pub fn run_search(&mut self) {
        self.search_job = None;
        self.results = Results::default();
        self.selected = 0;
        self.search_pending = false;
        self.last_query = self.popup_input.clone();
        self.search_buffer = if self.popup == Some(Popup::Find) {
            Some(self.active)
        } else {
            None
        };
        if self.popup_input.is_empty() {
            return;
        }
        if self.popup == Some(Popup::Find) {
            if !self.has_document {
                return;
            }
            match search::compile(&self.popup_input, &self.search_options) {
                Ok(re) => {
                    let path = self
                        .doc()
                        .path
                        .clone()
                        .unwrap_or_else(|| self.root.join("[untitled]"));
                    self.results.hits = search::in_text(&path, &self.doc().text(), &re, 5001);
                    if self.results.hits.len() > 5000 {
                        self.results.hits.truncate(5000);
                        self.results.truncated = true;
                    }
                    self.results.scanned = 1;
                }
                Err(e) => self.results.error = Some(e),
            }
        } else {
            let open = self
                .docs
                .iter()
                .filter_map(|d| d.path.clone().map(|p| (p, d.text())))
                .collect();
            self.search_job = Some(search::start(
                self.root.clone(),
                Some(self.popup_input.clone()),
                self.search_options.clone(),
                open,
            ));
        }
    }
    pub fn jump_hit(&mut self, hit: Hit) {
        self.markdown.focused = false;
        if self.markdown_mode == MarkdownMode::Preview {
            self.set_markdown_mode(MarkdownMode::Split);
        }
        if let Some(index) = self.search_buffer.filter(|&i| i < self.docs.len()) {
            self.active = index;
            self.has_document = true;
        } else {
            self.open_file(hit.path.clone());
            if self.doc().path.as_ref() != Some(&hit.path) {
                return;
            }
        }
        let row = hit.row.min(u16::MAX as usize) as u16;
        let col = hit.col.min(u16::MAX as usize) as u16;
        let end = hit.end.min(u16::MAX as usize) as u16;
        self.doc_mut().textarea.cancel_selection();
        self.doc_mut()
            .textarea
            .move_cursor(CursorMove::Jump(row, col));
        self.doc_mut().textarea.start_selection();
        self.doc_mut()
            .textarea
            .move_cursor(CursorMove::Jump(row, end));
        self.reveal(Focus::Editor);
        self.popup = None;
        self.status = format!(
            "MATCH  {}:{}:{}",
            hit.path
                .strip_prefix(&self.root)
                .unwrap_or(&hit.path)
                .display(),
            hit.row + 1,
            hit.col + 1
        );
    }
    pub fn match_step(&mut self, back: bool) {
        if self.results.hits.is_empty() {
            self.status = "No search results. Ctrl+F file, Ctrl+G project".into();
            return;
        }
        self.selected = if back {
            (self.selected + self.results.hits.len() - 1) % self.results.hits.len()
        } else {
            (self.selected + 1) % self.results.hits.len()
        };
        self.jump_hit(self.results.hits[self.selected].clone());
    }
    pub fn ensure_shell(&mut self) {
        if self.shell.is_none() && self.shell_error.is_none() {
            match Shell::new(&self.root) {
                Ok(s) => self.shell = Some(s),
                Err(e) => self.shell_error = Some(format!("{e:#}")),
            }
        }
    }
    pub fn send_ai(&mut self) {
        if self.chat_input.trim().is_empty() || self.chat_rx.is_some() {
            return;
        }
        let config = match ai::RequestConfig::from_settings(&self.settings) {
            Ok(c) => c,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        let prompt = std::mem::take(&mut self.chat_input);
        self.chat.push(ChatMessage {
            role: "user",
            content: prompt,
        });
        let file = if self.settings.ai_context && self.has_document {
            Some((self.doc().title.clone(), self.doc().text()))
        } else {
            None
        };
        self.chat_rx = Some(ai::ask(self.chat.clone(), file, config));
        self.chat_scroll = usize::MAX;
        self.status = "AI request in progress...".into();
    }
    pub fn tick(&mut self) {
        if let Some(s) = &mut self.shell {
            s.tick();
        }
        if let Some(result) = self.chat_rx.as_ref().and_then(|r| r.try_recv().ok()) {
            self.chat_rx = None;
            let text = match result {
                Ok(s) => {
                    self.status = "AI response received".into();
                    s
                }
                Err(e) => {
                    self.status = "AI request failed".into();
                    format!("AI error: {e}")
                }
            };
            self.chat.push(ChatMessage {
                role: "assistant",
                content: text,
            });
            self.chat_scroll = usize::MAX;
        }
        if let Some(result) = self.models_rx.as_ref().and_then(|r| r.try_recv().ok()) {
            self.models_rx = None;
            match result {
                Ok(models) if self.settings_original.is_some() => {
                    self.settings.ai_models = models;
                    self.status = "Models refreshed. Choose Model; Ctrl+S saves settings.".into();
                }
                Err(e) => self.status = format!("Models: {e}"),
                _ => {}
            }
        }
        if let Some(result) = self.index_job.as_ref().and_then(|j| j.rx.try_recv().ok()) {
            self.index_job = None;
            self.status = format!(
                "{} indexed files{}",
                result.files.len(),
                if result.truncated {
                    " (index limit reached)"
                } else {
                    ""
                }
            );
            self.file_index = result.files;
            self.filter_files();
        }
        if let Some(result) = self.search_job.as_ref().and_then(|j| j.rx.try_recv().ok()) {
            self.search_job = None;
            self.results = result;
            self.selected = 0;
        }
    }
    pub fn command(&mut self, c: Command) {
        self.popup = None;
        if matches!(
            c,
            Command::Cut | Command::Paste | Command::Undo | Command::Redo | Command::SelectAll
        ) {
            self.markdown.focused = false;
            if self.markdown_mode == MarkdownMode::Preview {
                self.set_markdown_mode(MarkdownMode::Split);
            }
        }
        match c {
            Command::New => self.new_file(),
            Command::Open => self.open_popup(Popup::Open),
            Command::Files => self.open_popup(Popup::Files),
            Command::Save => self.save(),
            Command::SaveAs => {
                if self.has_document {
                    self.open_popup(Popup::SaveAs)
                }
            }
            Command::Close => self.close_file(),
            Command::Quit => self.quit(),
            Command::Undo => {
                self.doc_mut().textarea.undo();
                self.doc_mut().highlight_dirty = true;
            }
            Command::Redo => {
                self.doc_mut().textarea.redo();
                self.doc_mut().highlight_dirty = true;
            }
            Command::Copy => {
                if self.doc().textarea.selection_range().is_some() {
                    self.doc_mut().textarea.copy();
                    self.clipboard = self.doc().textarea.yank_text().to_owned();
                }
            }
            Command::Cut => {
                if self.doc().textarea.selection_range().is_some() {
                    self.doc_mut().textarea.cut();
                    self.clipboard = self.doc().textarea.yank_text().to_owned();
                    self.doc_mut().highlight_dirty = true;
                }
            }
            Command::Paste => {
                if !self.has_document {
                    self.new_file();
                }
                let text = self.clipboard.clone();
                self.doc_mut().textarea.insert_str(text);
                self.doc_mut().highlight_dirty = true;
            }
            Command::SelectAll => self.doc_mut().textarea.select_all(),
            Command::Find => self.open_popup(Popup::Find),
            Command::Project => self.open_popup(Popup::Project),
            Command::NextMatch => self.match_step(false),
            Command::PrevMatch => self.match_step(true),
            Command::NextTab => self.next_tab(false),
            Command::PrevTab => self.next_tab(true),
            Command::Toggle(p) => self.toggle(p),
            Command::Map => self.show_minimap = !self.show_minimap,
            Command::MarkdownToggle | Command::MarkdownFull | Command::MarkdownFocus => {
                if !self.has_document || !self.doc().is_markdown() {
                    self.status = "Open a .md file to use Markdown preview".into();
                    return;
                }
                self.reveal(Focus::Editor);
                match c {
                    Command::MarkdownToggle => {
                        let mode = if self.markdown_mode == MarkdownMode::Source {
                            MarkdownMode::Split
                        } else {
                            MarkdownMode::Source
                        };
                        self.set_markdown_mode(mode);
                        self.markdown.focused = false;
                    }
                    Command::MarkdownFull => {
                        let mode = if self.markdown_mode == MarkdownMode::Preview {
                            MarkdownMode::Split
                        } else {
                            MarkdownMode::Preview
                        };
                        self.set_markdown_mode(mode);
                        self.markdown.focused = self.markdown_mode == MarkdownMode::Preview;
                    }
                    _ => {
                        if self.markdown_mode == MarkdownMode::Source {
                            self.set_markdown_mode(MarkdownMode::Split);
                        }
                        self.markdown.focused = !self.markdown.focused;
                        if self.markdown_mode == MarkdownMode::Preview && !self.markdown.focused {
                            self.set_markdown_mode(MarkdownMode::Split);
                        }
                    }
                }
                self.status =
                    "Markdown: Ctrl+K Ctrl+W split/off | Ctrl+K Ctrl+L full | Ctrl+K Ctrl+Y focus"
                        .into();
            }
            Command::Zoom => {
                if !self.enabled(self.focus) {
                    self.reveal(Focus::Editor);
                }
                self.zoom = if self.zoom.is_some() {
                    None
                } else {
                    Some(self.focus)
                }
            }
            Command::Restore => {
                self.set_markdown_mode(if self.settings.markdown_preview {
                    MarkdownMode::Split
                } else {
                    MarkdownMode::Source
                });
                self.markdown.focused = false;
                self.zoom = None;
                self.show_tree = self.settings.tree;
                self.show_editor = self.settings.editor;
                self.show_chat = self.settings.chat;
                self.show_terminal = self.settings.terminal;
                self.show_minimap = self.settings.minimap;
                self.normalize_focus();
            }
            Command::Settings => self.open_popup(Popup::Settings),
            Command::Help => self.open_popup(Popup::Help),
            Command::About => self.open_popup(Popup::About),
            Command::Refresh => self.tree.refresh(),
        }
    }
    pub fn menu_items(&self) -> Vec<(String, &'static str, Command)> {
        let row = |a: &str, b, c| (a.to_string(), b, c);
        match self.menu {
            0 => vec![
                row("New file", "Ctrl+N", Command::New),
                row("Open file / folder...", "Ctrl+O", Command::Open),
                row("Quick Open...", "Ctrl+P", Command::Files),
                row("Save", "Ctrl+S", Command::Save),
                row("Save As...", "Ctrl+K Ctrl+S", Command::SaveAs),
                row("Close file", "Ctrl+W", Command::Close),
                row("Next file", "Ctrl+K Ctrl+D", Command::NextTab),
                row("Previous file", "Ctrl+K Ctrl+U", Command::PrevTab),
                row("Quit", "Ctrl+Q", Command::Quit),
            ],
            1 => vec![
                row("Undo", "Ctrl+U", Command::Undo),
                row("Redo", "Ctrl+R", Command::Redo),
                row("Copy selection", "Ctrl+C", Command::Copy),
                row("Cut selection", "Ctrl+X", Command::Cut),
                row("Paste internal clipboard", "Ctrl+V", Command::Paste),
                row("Select all", "Ctrl+A", Command::SelectAll),
                row("Find in file...", "Ctrl+F", Command::Find),
                row("Search project...", "Ctrl+G", Command::Project),
                row("Next match", "Ctrl+K Ctrl+N", Command::NextMatch),
                row("Previous match", "Ctrl+K Ctrl+P", Command::PrevMatch),
            ],
            2 => {
                let mut rows = Vec::new();
                for (label, p, k) in [
                    ("Explorer", Focus::Tree, "Ctrl+K Ctrl+B"),
                    ("Editor", Focus::Editor, "Ctrl+K Ctrl+E"),
                    ("AI chat", Focus::Chat, "Ctrl+K Ctrl+A"),
                    ("Terminal", Focus::Terminal, "Ctrl+K Ctrl+T"),
                ] {
                    rows.push((
                        format!("[{}] {label}", if self.enabled(p) { "x" } else { " " }),
                        k,
                        Command::Toggle(p),
                    ));
                }
                rows.push((
                    format!("[{}] Minimap", if self.show_minimap { "x" } else { " " }),
                    "Ctrl+K Ctrl+M",
                    Command::Map,
                ));
                rows.extend([
                    row(
                        "Markdown preview / source",
                        "Ctrl+K Ctrl+W",
                        Command::MarkdownToggle,
                    ),
                    row(
                        "Full Markdown preview / split",
                        "Ctrl+K Ctrl+L",
                        Command::MarkdownFull,
                    ),
                    row(
                        "Focus Markdown preview / source",
                        "Ctrl+K Ctrl+Y",
                        Command::MarkdownFocus,
                    ),
                    row(
                        "Maximize / restore active panel",
                        "Ctrl+K Ctrl+Z",
                        Command::Zoom,
                    ),
                    row("Restore default layout", "Ctrl+K Ctrl+R", Command::Restore),
                    row("Refresh explorer", "", Command::Refresh),
                ]);
                rows
            }
            3 => vec![row("Preferences...", "Ctrl+K Ctrl+O", Command::Settings)],
            _ => vec![
                row("Keyboard commands", "Ctrl+K Ctrl+H", Command::Help),
                row("About GEditor", "Ctrl+K Ctrl+I", Command::About),
            ],
        }
    }
    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let ch = if let KeyCode::Char(c) = key.code {
            Some(c.to_ascii_lowercase())
        } else {
            None
        };
        if self.popup.is_some() {
            self.handle_popup(key);
            return;
        }
        if self.prefix {
            self.prefix = false;
            // Legacy terminals encode Ctrl+I/M/H as Tab/Enter/Backspace.
            // Inside an explicit command prefix those bytes are unambiguous.
            let legacy = match key.code {
                KeyCode::Tab => Some('i'),
                KeyCode::Enter => Some('m'),
                KeyCode::Backspace => Some('h'),
                _ => None,
            };
            let ch = ch.or(legacy);
            let ctrl = ctrl || legacy.is_some();
            if key.code == KeyCode::Esc {
                self.status = "Command cancelled".into();
                return;
            }
            if !ctrl {
                self.status =
                    "Press Ctrl+K, then another Ctrl shortcut. Ctrl+E opens the menu.".into();
                return;
            }
            let command = match ch {
                Some('b') => Some(Command::Toggle(Focus::Tree)),
                Some('e') => Some(Command::Toggle(Focus::Editor)),
                Some('a') => Some(Command::Toggle(Focus::Chat)),
                Some('t') => Some(Command::Toggle(Focus::Terminal)),
                Some('m') => Some(Command::Map),
                Some('w') => Some(Command::MarkdownToggle),
                Some('l') => Some(Command::MarkdownFull),
                Some('y') => Some(Command::MarkdownFocus),
                Some('z') => Some(Command::Zoom),
                Some('r') => Some(Command::Restore),
                Some('s') => Some(Command::SaveAs),
                Some('o') => Some(Command::Settings),
                Some('h') => Some(Command::Help),
                Some('i') => Some(Command::About),
                Some('n') => Some(Command::NextMatch),
                Some('p') => Some(Command::PrevMatch),
                Some('d') => Some(Command::NextTab),
                Some('u') => Some(Command::PrevTab),
                _ => None,
            };
            if let Some(c) = command {
                self.command(c);
            } else {
                match ch {
                    Some('f') => self.cycle(shift),
                    Some('v') => self.open_popup(Popup::Menu),
                    Some('c') => self.toggle(self.focus),
                    Some('k') => {
                        if let Some(s) = &mut self.shell {
                            let _ = s.handle_key(key);
                        }
                    }
                    _ => self.status = "Unknown command. Ctrl+K Ctrl+H for help.".into(),
                }
            }
            return;
        }
        if ctrl && ch == Some('k') {
            self.prefix = true;
            self.status="Ctrl+K: [B] sidebar [E] editor [A] AI [T] shell [M] map [Z] maximize [O] settings [H] help".into();
            return;
        }
        if ctrl && (key.code == KeyCode::Tab || key.code == KeyCode::BackTab) {
            self.cycle(shift || key.code == KeyCode::BackTab);
            return;
        }
        if ctrl && ch == Some('q') {
            self.quit();
            return;
        }
        self.quit_armed = false;
        // Keep native shell editing controls, including Ctrl+C/D/L/E, available.
        if self.focus == Focus::Terminal {
            self.ensure_shell();
            if let Some(s) = &mut self.shell
                && let Err(e) = s.handle_key(key)
            {
                self.status = format!("Shell: {e}");
            }
            return;
        }
        if ctrl {
            let c = match ch {
                Some('e') => {
                    self.open_popup(Popup::Menu);
                    return;
                }
                Some('n') => Some(Command::New),
                Some('o') => Some(Command::Open),
                Some('p') => Some(Command::Files),
                Some('s') => Some(Command::Save),
                Some('w') => Some(Command::Close),
                Some('f') => Some(Command::Find),
                Some('g') => Some(Command::Project),
                Some('u') if self.focus == Focus::Editor => Some(Command::Undo),
                Some('r') if self.focus == Focus::Editor => Some(Command::Redo),
                Some('c') if self.focus == Focus::Editor => Some(Command::Copy),
                Some('x') if self.focus == Focus::Editor => Some(Command::Cut),
                Some('v') if self.focus == Focus::Editor => Some(Command::Paste),
                Some('a') if self.focus == Focus::Editor => Some(Command::SelectAll),
                _ => None,
            };
            if let Some(c) = c {
                self.command(c);
                return;
            }
        }
        if self.focus == Focus::Editor
            && self.has_document
            && self.doc().is_markdown()
            && self.markdown_mode != MarkdownMode::Source
            && self.markdown.focused
        {
            let h = self.markdown_rect.height.saturating_sub(3) as usize;
            match key.code {
                KeyCode::Up => self.markdown.scroll(-1, h),
                KeyCode::Down => self.markdown.scroll(1, h),
                KeyCode::PageUp => self.markdown.scroll(-(h.max(1) as isize), h),
                KeyCode::PageDown => self.markdown.scroll(h.max(1) as isize, h),
                KeyCode::Home => self.markdown.top = 0,
                KeyCode::End => {
                    self.markdown.top = usize::MAX;
                    self.markdown.clamp(h);
                }
                KeyCode::Esc => {
                    self.markdown.focused = false;
                    if self.markdown_mode == MarkdownMode::Preview {
                        self.set_markdown_mode(MarkdownMode::Split);
                    }
                }
                KeyCode::Char(_) | KeyCode::Enter | KeyCode::Backspace | KeyCode::Delete => {
                    self.status =
                        "Markdown preview is read-only. Ctrl+K Ctrl+Y returns to source.".into()
                }
                _ => {}
            }
            return;
        }
        if key.code == KeyCode::Esc {
            self.doc_mut().textarea.cancel_selection();
            return;
        }
        match self.focus {
            Focus::Tree => match key.code {
                KeyCode::Down => {
                    self.tree.selected =
                        (self.tree.selected + 1).min(self.tree.entries.len().saturating_sub(1))
                }
                KeyCode::Up => self.tree.selected = self.tree.selected.saturating_sub(1),
                KeyCode::Left => self.tree.collapse_or_parent(),
                KeyCode::Enter => {
                    if let Some(p) = self.tree.toggle_or_open() {
                        self.open_file(p)
                    }
                }
                KeyCode::Right => {
                    if let Some(p) = self.tree.expand_or_open() {
                        self.open_file(p)
                    }
                }
                _ => {}
            },
            Focus::Editor => {
                if !self.has_document
                    && matches!(key.code, KeyCode::Char(_) | KeyCode::Enter | KeyCode::Tab)
                    && !ctrl
                {
                    self.new_file();
                }
                if self.has_document && self.doc_mut().textarea.input(key) {
                    self.doc_mut().highlight_dirty = true;
                }
            }
            Focus::Chat => match key.code {
                KeyCode::Enter if shift => self.chat_input.push('\n'),
                KeyCode::Enter => self.send_ai(),
                KeyCode::Backspace => {
                    self.chat_input.pop();
                }
                KeyCode::Char(c) if !ctrl => self.chat_input.push(c),
                KeyCode::PageUp => self.chat_scroll = self.chat_scroll.saturating_sub(8),
                KeyCode::PageDown => self.chat_scroll = self.chat_scroll.saturating_add(8),
                _ => {}
            },
            Focus::Terminal => {}
        }
    }
    pub fn paste(&mut self, text: &str) {
        if self.popup == Some(Popup::Settings) {
            if self.editing {
                self.edit_text.push_str(text.trim_end_matches(['\r', '\n']));
            }
            return;
        }
        if matches!(
            self.popup,
            Some(Popup::Open | Popup::SaveAs | Popup::Files | Popup::Find | Popup::Project)
        ) {
            let text = text.lines().next().unwrap_or_default();
            let (input, cursor) = if self.search_glob_edit {
                (&mut self.search_options.glob, &mut self.glob_cursor)
            } else {
                (&mut self.popup_input, &mut self.input_cursor)
            };
            *cursor = (*cursor).min(input.len());
            input.insert_str(*cursor, text);
            *cursor += text.len();
            self.query_changed();
            return;
        }
        if self.popup.is_some() {
            return;
        }
        match self.focus {
            Focus::Editor => {
                if self.markdown.focused
                    && self.markdown_mode != MarkdownMode::Source
                    && self.has_document
                    && self.doc().is_markdown()
                {
                    self.status = "Preview is read-only. Ctrl+K Ctrl+Y returns to source.".into();
                    return;
                }
                if !self.has_document {
                    self.new_file();
                }
                self.doc_mut().textarea.insert_str(text);
                self.doc_mut().highlight_dirty = true;
            }
            Focus::Chat => self.chat_input.push_str(text),
            Focus::Terminal => {
                if let Some(s) = &mut self.shell {
                    let _ = s.paste(text);
                }
            }
            _ => {}
        }
    }
    pub fn query_changed(&mut self) {
        self.selected = 0;
        if self.popup == Some(Popup::Files) {
            self.filter_files();
        }
        if matches!(self.popup, Some(Popup::Find | Popup::Project)) {
            self.search_job = None;
            self.results = Results::default();
            self.search_pending = true;
            if self.popup == Some(Popup::Find) {
                self.run_search();
            }
        }
    }
    fn handle_popup(&mut self, key: KeyEvent) {
        let mode = self.popup.unwrap();
        if mode == Popup::Settings {
            self.handle_settings(key);
            return;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let ch = if let KeyCode::Char(c) = key.code {
            Some(c.to_ascii_lowercase())
        } else {
            None
        };
        if key.code == KeyCode::Esc {
            self.popup = None;
            self.search_job = None;
            self.index_job = None;
            self.search_glob_edit = false;
            return;
        }
        if matches!(mode, Popup::Help | Popup::About) {
            match key.code {
                KeyCode::Enter => self.popup = None,
                KeyCode::Down => self.selected = (self.selected + 1).min(45),
                KeyCode::Up => self.selected = self.selected.saturating_sub(1),
                KeyCode::PageDown => self.selected = (self.selected + 10).min(45),
                KeyCode::PageUp => self.selected = self.selected.saturating_sub(10),
                _ => {}
            }
            return;
        }
        if mode == Popup::Menu {
            if ctrl && ch.is_some() && ch != Some('e') {
                self.popup = None;
                self.handle_key(key);
                return;
            }
            let len = self.menu_items().len();
            match key.code {
                KeyCode::Left => {
                    self.menu = (self.menu + 4) % 5;
                    self.menu_row = 0;
                }
                KeyCode::Right | KeyCode::Tab => {
                    self.menu = (self.menu + 1) % 5;
                    self.menu_row = 0;
                }
                KeyCode::Up => self.menu_row = (self.menu_row + len - 1) % len,
                KeyCode::Down => self.menu_row = (self.menu_row + 1) % len,
                KeyCode::Enter => self.command(self.menu_items()[self.menu_row].2),
                KeyCode::Char('e') if ctrl => self.popup = None,
                _ => {}
            }
            return;
        }
        if ctrl && ch == Some('u') {
            if self.search_glob_edit {
                self.search_options.glob.clear();
                self.glob_cursor = 0;
            } else {
                self.popup_input.clear();
                self.input_cursor = 0;
            }
            self.query_changed();
            return;
        }
        let (input, cursor) = if self.search_glob_edit {
            (&mut self.search_options.glob, &mut self.glob_cursor)
        } else {
            (&mut self.popup_input, &mut self.input_cursor)
        };
        *cursor = (*cursor).min(input.len());
        if matches!(
            key.code,
            KeyCode::Left | KeyCode::Right | KeyCode::Home | KeyCode::End | KeyCode::Delete
        ) {
            match key.code {
                KeyCode::Home => *cursor = 0,
                KeyCode::End => *cursor = input.len(),
                KeyCode::Left => {
                    *cursor = input[..*cursor]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0)
                }
                KeyCode::Right => {
                    *cursor += input[*cursor..]
                        .chars()
                        .next()
                        .map(char::len_utf8)
                        .unwrap_or(0)
                }
                KeyCode::Delete => {
                    if let Some(c) = input[*cursor..].chars().next() {
                        let end = *cursor + c.len_utf8();
                        input.replace_range(*cursor..end, "");
                        self.query_changed();
                    }
                }
                _ => {}
            }
            return;
        }
        if ctrl && matches!(mode, Popup::Find | Popup::Project) {
            let mut changed = true;
            match ch {
                Some('c') => self.search_options.case = !self.search_options.case,
                Some('w') => self.search_options.word = !self.search_options.word,
                Some('r') => self.search_options.regex = !self.search_options.regex,
                Some('d') => self.search_options.hidden = !self.search_options.hidden,
                Some('a') => self.search_options.ignored = !self.search_options.ignored,
                Some('t') => {
                    self.search_glob_edit = !self.search_glob_edit;
                    changed = false
                }
                Some('s') => {
                    self.run_search();
                    changed = false
                }
                _ => changed = false,
            }
            if changed {
                self.query_changed();
            }
            return;
        }
        let count = if mode == Popup::Files {
            self.file_hits.len()
        } else {
            self.results.hits.len()
        };
        match key.code {
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => self.selected = (self.selected + 1).min(count.saturating_sub(1)),
            KeyCode::PageUp => self.selected = self.selected.saturating_sub(10),
            KeyCode::PageDown => self.selected = (self.selected + 10).min(count.saturating_sub(1)),
            KeyCode::Backspace => {
                let (input, cursor) = if self.search_glob_edit {
                    (&mut self.search_options.glob, &mut self.glob_cursor)
                } else {
                    (&mut self.popup_input, &mut self.input_cursor)
                };
                *cursor = (*cursor).min(input.len());
                if *cursor > 0 {
                    let previous = input[..*cursor]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    input.replace_range(previous..*cursor, "");
                    *cursor = previous;
                    self.query_changed();
                }
            }
            KeyCode::Char(c) if !ctrl => {
                let (input, cursor) = if self.search_glob_edit {
                    (&mut self.search_options.glob, &mut self.glob_cursor)
                } else {
                    (&mut self.popup_input, &mut self.input_cursor)
                };
                *cursor = (*cursor).min(input.len());
                input.insert(*cursor, c);
                *cursor += c.len_utf8();
                self.query_changed();
            }
            KeyCode::Enter => match mode {
                Popup::Files => {
                    if let Some(path) = self.file_hits.get(self.selected).cloned() {
                        self.popup = None;
                        self.open_file(path);
                    }
                }
                Popup::Find | Popup::Project => {
                    if self.search_pending {
                        self.run_search();
                    } else if let Some(hit) = self.results.hits.get(self.selected).cloned() {
                        self.jump_hit(hit);
                    }
                }
                Popup::SaveAs => {
                    let path = self.expand_path(&self.popup_input);
                    if self.popup_input.trim().is_empty() {
                        return;
                    }
                    match self.doc_mut().save_as(path) {
                        Ok(p) => {
                            self.status = format!("SAVED  {}", p.display());
                            self.popup = None;
                            self.tree.refresh();
                            self.doc_mut().highlight_dirty = true;
                        }
                        Err(e) => self.status = format!("SAVE FAILED  {e:#}"),
                    }
                }
                Popup::Open => {
                    let path = self.expand_path(&self.popup_input);
                    if path.is_dir() {
                        if self.docs.iter().any(Document::is_dirty) {
                            self.status = "Save unsaved files before changing the workspace".into();
                            return;
                        }
                        if let Ok(root) = fs::canonicalize(path) {
                            self.root = root.clone();
                            self.tree = FileTree::new(root);
                            self.docs = vec![Document::empty("untitled")];
                            self.settings.style_doc(&mut self.docs[0]);
                            self.has_document = false;
                            self.active = 0;
                            self.shell = None;
                            self.shell_error = None;
                            self.file_index.clear();
                            self.results = Results::default();
                            self.popup = None;
                        }
                    } else {
                        self.popup = None;
                        self.open_file(path);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    fn expand_path(&self, input: &str) -> PathBuf {
        let text = input.trim();
        if let Some(rest) = text.strip_prefix("~/") {
            return PathBuf::from(
                env::var_os("HOME")
                    .or_else(|| env::var_os("USERPROFILE"))
                    .unwrap_or_default(),
            )
            .join(rest);
        }
        let p = PathBuf::from(text);
        if p.is_absolute() {
            p
        } else {
            self.root.join(p)
        }
    }
    pub fn fields(&self) -> &'static [Field] {
        match self.settings_tab {
            0 => APPEARANCE,
            1 => PANELS,
            2 => AI,
            _ => GENERAL,
        }
    }
    pub fn field_row(&self, f: Field) -> (&'static str, String, &'static str) {
        let s = &self.settings;
        let on = |b| {
            if b {
                "On".to_string()
            } else {
                "Off".to_string()
            }
        };
        let color = |v: &String| {
            if v.is_empty() {
                "Theme default".into()
            } else {
                v.clone()
            }
        };
        match f {
            Field::Theme => (
                "Theme",
                s.theme.clone(),
                "Enter opens a dropdown; preview is immediate",
            ),
            Field::ColorMode => (
                "Color support",
                s.color_mode.clone(),
                "Auto, true color, ANSI 256, ANSI 16 or monochrome",
            ),
            Field::Accent => (
                "Accent color",
                color(&s.accent),
                "Enter a #RRGGBB color; empty restores theme default",
            ),
            Field::Background => (
                "Background",
                color(&s.background),
                "Enter a #RRGGBB color; empty restores theme default",
            ),
            Field::Foreground => (
                "Text color",
                color(&s.foreground),
                "Enter a #RRGGBB color; empty restores theme default",
            ),
            Field::Ascii => (
                "ASCII borders",
                on(s.ascii),
                "Use simple ASCII borders and minimap for limited fonts",
            ),
            Field::Clock => (
                "Large home clock",
                on(s.clock_large),
                "Hours : minutes : seconds when no file is open",
            ),
            Field::TabWidth => (
                "Tab width",
                s.tab_width.to_string(),
                "Left/right adjusts indentation width",
            ),
            Field::LineNumbers => (
                "Line numbers",
                on(s.line_numbers),
                "Show line numbers beside code",
            ),
            Field::Font => (
                "Font size",
                "Controlled by terminal".into(),
                "TUI apps cannot set fonts. Use your terminal's font/zoom settings (on Mac: Cmd +/-).",
            ),
            Field::Tree => (
                "Explorer by default",
                on(s.tree),
                "Saved starting visibility; View menu controls this session",
            ),
            Field::Editor => (
                "Editor by default",
                on(s.editor),
                "Saved starting visibility",
            ),
            Field::Chat => (
                "AI chat by default",
                on(s.chat),
                "Saved starting visibility",
            ),
            Field::Terminal => (
                "Terminal by default",
                on(s.terminal),
                "Saved starting visibility",
            ),
            Field::Markdown => (
                "Markdown on .md open",
                on(s.markdown_preview),
                "Open .md files with a live preview; View controls this session",
            ),
            Field::Minimap => (
                "Minimap by default",
                on(s.minimap),
                "Whole-file density overview with viewport and cursor markers",
            ),
            Field::TreeWidth => (
                "Explorer width",
                s.sidebar_width.to_string(),
                "16-50 terminal columns; Left/right to adjust",
            ),
            Field::ChatWidth => (
                "AI width",
                s.chat_width.to_string(),
                "24-80 terminal columns; Left/right to adjust",
            ),
            Field::TermHeight => (
                "Terminal height",
                format!("{}%", s.terminal_percent),
                "15-65 percent of workspace height",
            ),
            Field::MapWidth => (
                "Minimap width",
                s.minimap_width.to_string(),
                "5-20 columns; auto hides on small screens",
            ),
            Field::Model => (
                "Model",
                s.ai_model.clone(),
                "Choose from cached models. Refresh models loads your key's actual catalog.",
            ),
            Field::CustomModel => (
                "Custom model ID",
                s.ai_model.clone(),
                "Enter any text-capable model ID supported by your endpoint",
            ),
            Field::FetchModels => (
                "Refresh models",
                "Fetch from API".into(),
                "Explicit network request to /models; no file content is sent",
            ),
            Field::Profile => (
                "Active API key",
                if s.ai_profile == 0 {
                    "Environment: OPENAI_API_KEY".into()
                } else {
                    s.keys
                        .get(s.ai_profile - 1)
                        .map(|p| p.name.clone())
                        .unwrap_or_default()
                },
                "Dropdown selects environment or a locally saved key profile",
            ),
            Field::ProfileName => (
                "Profile label",
                if s.ai_profile == 0 {
                    "Select a saved profile first".into()
                } else {
                    s.keys[s.ai_profile - 1].name.clone()
                },
                "Personal, Work and Other profiles can be renamed",
            ),
            Field::ApiKey => (
                "Profile API key",
                if s.ai_profile == 0 {
                    "Read from environment".into()
                } else if s.keys[s.ai_profile - 1].key.is_empty() {
                    "Not set".into()
                } else {
                    "******** (saved locally)".into()
                },
                "Enter a replacement; blank clears. Stored locally in settings.json (0600).",
            ),
            Field::BaseUrl => (
                "API base URL",
                s.ai_base.clone(),
                "Responses-compatible API; HTTPS required except localhost",
            ),
            Field::Context => (
                "Include file context",
                on(s.ai_context),
                "When enabled, sending a chat includes up to 16 KB of the active buffer",
            ),
            Field::Author => (
                "Author credit",
                s.author.clone(),
                "Shown in About; free software, Rust, MIT or Apache-2.0 license",
            ),
        }
    }
    fn preview_settings(&mut self) {
        for d in &mut self.docs {
            self.settings.style_doc(d);
        }
    }
    fn change_field(&mut self, f: Field, delta: i16) {
        let s = &mut self.settings;
        let adjust = |v: u16, min, max| ((v as i32 + delta as i32).clamp(min, max)) as u16;
        match f {
            Field::Ascii => s.ascii = !s.ascii,
            Field::Clock => s.clock_large = !s.clock_large,
            Field::LineNumbers => s.line_numbers = !s.line_numbers,
            Field::Tree => s.tree = !s.tree,
            Field::Editor => s.editor = !s.editor,
            Field::Chat => s.chat = !s.chat,
            Field::Terminal => s.terminal = !s.terminal,
            Field::Minimap => s.minimap = !s.minimap,
            Field::Markdown => s.markdown_preview = !s.markdown_preview,
            Field::Context => s.ai_context = !s.ai_context,
            Field::TreeWidth => s.sidebar_width = adjust(s.sidebar_width, 16, 50),
            Field::ChatWidth => s.chat_width = adjust(s.chat_width, 24, 80),
            Field::TermHeight => s.terminal_percent = adjust(s.terminal_percent, 15, 65),
            Field::MapWidth => s.minimap_width = adjust(s.minimap_width, 5, 20),
            Field::TabWidth => s.tab_width = adjust(s.tab_width as u16, 1, 8) as u8,
            _ => {}
        }
        self.preview_settings();
    }
    fn activate_field(&mut self) {
        let f = self.fields()[self.settings_row];
        let items = match f {
            Field::Theme => vec![
                "Midnight".into(),
                "Light".into(),
                "Dracula".into(),
                "Terminal".into(),
            ],
            Field::ColorMode => vec![
                "Auto".into(),
                "True color".into(),
                "256 colors".into(),
                "16 colors".into(),
                "Mono".into(),
            ],
            Field::Model => self.settings.ai_models.clone(),
            Field::Profile => std::iter::once("Environment: OPENAI_API_KEY".into())
                .chain(self.settings.keys.iter().map(|p| p.name.clone()))
                .collect(),
            _ => Vec::new(),
        };
        if !items.is_empty() {
            let value = self.field_row(f).1;
            let selected = items.iter().position(|v| v == &value).unwrap_or(0);
            self.dropdown = Some(Dropdown {
                field: f,
                items,
                selected,
            });
            return;
        }
        if f == Field::FetchModels {
            match ai::RequestConfig::from_settings(&self.settings) {
                Ok(c) => {
                    self.models_rx = Some(ai::models(c));
                    self.status = "Loading models...".into();
                }
                Err(e) => self.status = e,
            }
            return;
        }
        let text = match f {
            Field::Accent => Some(self.settings.accent.clone()),
            Field::Background => Some(self.settings.background.clone()),
            Field::Foreground => Some(self.settings.foreground.clone()),
            Field::CustomModel => Some(self.settings.ai_model.clone()),
            Field::BaseUrl => Some(self.settings.ai_base.clone()),
            Field::Author => Some(self.settings.author.clone()),
            Field::ProfileName if self.settings.ai_profile > 0 => Some(
                self.settings.keys[self.settings.ai_profile - 1]
                    .name
                    .clone(),
            ),
            Field::ApiKey if self.settings.ai_profile > 0 => Some(String::new()),
            _ => None,
        };
        if let Some(text) = text {
            self.edit_text = text;
            self.editing = true;
        } else {
            self.change_field(f, 1);
        }
    }
    fn commit_edit(&mut self) -> bool {
        let f = self.fields()[self.settings_row];
        let value = self.edit_text.trim().to_owned();
        if matches!(f, Field::Accent | Field::Background | Field::Foreground)
            && !value.is_empty()
            && crate::config::parse_hex(&value).is_none()
        {
            self.status = "Invalid color: use #RRGGBB or empty".into();
            return false;
        }
        match f {
            Field::Accent => self.settings.accent = value,
            Field::Background => self.settings.background = value,
            Field::Foreground => self.settings.foreground = value,
            Field::CustomModel => {
                if value.is_empty() {
                    self.status = "Model ID is required".into();
                    return false;
                }
                self.settings.ai_model = value.clone();
                if !self.settings.ai_models.contains(&value) {
                    self.settings.ai_models.push(value);
                }
            }
            Field::BaseUrl => self.settings.ai_base = value,
            Field::Author => self.settings.author = value,
            Field::ProfileName => self.settings.keys[self.settings.ai_profile - 1].name = value,
            Field::ApiKey => self.settings.keys[self.settings.ai_profile - 1].key = value,
            _ => {}
        }
        self.edit_text.clear();
        self.editing = false;
        self.preview_settings();
        true
    }
    fn handle_settings(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        if let Some(drop) = &mut self.dropdown {
            match key.code {
                KeyCode::Esc => self.dropdown = None,
                KeyCode::Up => drop.selected = drop.selected.saturating_sub(1),
                KeyCode::Down => drop.selected = (drop.selected + 1).min(drop.items.len() - 1),
                KeyCode::PageUp => drop.selected = drop.selected.saturating_sub(10),
                KeyCode::PageDown => drop.selected = (drop.selected + 10).min(drop.items.len() - 1),
                KeyCode::Enter => {
                    let d = self.dropdown.take().unwrap();
                    match d.field {
                        Field::Theme => self.settings.theme = d.items[d.selected].clone(),
                        Field::ColorMode => self.settings.color_mode = d.items[d.selected].clone(),
                        Field::Model => self.settings.ai_model = d.items[d.selected].clone(),
                        Field::Profile => self.settings.ai_profile = d.selected,
                        _ => {}
                    }
                    self.preview_settings();
                }
                _ => {}
            }
            return;
        }
        if ctrl && matches!(key.code, KeyCode::Char('s' | 'S')) {
            if self.editing && !self.commit_edit() {
                return;
            }
            match self.settings.save(&self.settings_path) {
                Ok(()) => {
                    self.settings_original = None;
                    self.popup = None;
                    self.show_tree = self.settings.tree;
                    self.show_editor = self.settings.editor;
                    self.show_chat = self.settings.chat;
                    self.show_terminal = self.settings.terminal;
                    self.show_minimap = self.settings.minimap;
                    let mode = if self.settings.markdown_preview {
                        MarkdownMode::Split
                    } else {
                        MarkdownMode::Source
                    };
                    self.set_markdown_mode(mode);
                    self.markdown.focused = false;
                    self.normalize_focus();
                    self.status = "Settings saved".into();
                }
                Err(e) => self.status = format!("Settings not saved: {e:#}"),
            }
            return;
        }
        if self.editing {
            match key.code {
                KeyCode::Esc => {
                    self.editing = false;
                    self.edit_text.clear();
                }
                KeyCode::Enter => {
                    self.commit_edit();
                }
                KeyCode::Backspace => {
                    self.edit_text.pop();
                }
                KeyCode::Char('u') if ctrl => self.edit_text.clear(),
                KeyCode::Char(c) if !ctrl => self.edit_text.push(c),
                _ => {}
            }
            return;
        }
        if ctrl && matches!(key.code, KeyCode::Char('n' | 'N' | 'p' | 'P')) {
            let back = matches!(key.code, KeyCode::Char('p' | 'P'));
            self.settings_tab = if back {
                (self.settings_tab + 3) % 4
            } else {
                (self.settings_tab + 1) % 4
            };
            self.settings_row = 0;
            return;
        }
        match key.code {
            KeyCode::Esc => {
                if let Some(original) = self.settings_original.take() {
                    self.settings = original;
                    self.preview_settings();
                }
                self.popup = None;
                self.models_rx = None;
                self.status = "Settings changes cancelled".into();
            }
            KeyCode::Tab if ctrl => {
                self.settings_tab = if shift {
                    (self.settings_tab + 3) % 4
                } else {
                    (self.settings_tab + 1) % 4
                };
                self.settings_row = 0;
            }
            KeyCode::BackTab if ctrl => {
                self.settings_tab = (self.settings_tab + 3) % 4;
                self.settings_row = 0;
            }
            KeyCode::Up => self.settings_row = self.settings_row.saturating_sub(1),
            KeyCode::Down | KeyCode::Tab => {
                self.settings_row = (self.settings_row + 1) % self.fields().len()
            }
            KeyCode::Left => {
                let f = self.fields()[self.settings_row];
                self.change_field(f, -1);
            }
            KeyCode::Right => {
                let f = self.fields()[self.settings_row];
                self.change_field(f, 1);
            }
            KeyCode::Enter | KeyCode::Char(' ') => self.activate_field(),
            _ => {}
        }
    }
    pub fn handle_mouse(&mut self, m: MouseEvent) {
        let click = matches!(m.kind, MouseEventKind::Down(MouseButton::Left));
        let contains = |r: Rect| r.contains(Position::new(m.column, m.row));
        if click
            && m.row == 0
            && let Some(i) = self.menu_rects.iter().position(|r| contains(*r))
        {
            if self.settings_original.is_some() {
                return;
            }
            self.menu = i;
            self.menu_row = 0;
            self.popup = Some(Popup::Menu);
            return;
        }
        if let Some(mode) = self.popup {
            if click && mode == Popup::Settings && self.dropdown.is_some() {
                if contains(self.dropdown_rect) {
                    let drop = self.dropdown.as_ref().unwrap();
                    let height = self.dropdown_rect.height as usize;
                    let start = drop.selected.saturating_sub(height.saturating_sub(1) / 2);
                    let index = start + (m.row - self.dropdown_rect.y) as usize;
                    if index < drop.items.len() {
                        self.dropdown.as_mut().unwrap().selected = index;
                        self.handle_settings(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
                    }
                }
                return;
            }
            if click {
                if mode == Popup::Menu && contains(self.result_rect) {
                    let i = m.row.saturating_sub(self.result_rect.y) as usize;
                    if let Some(item) = self.menu_items().get(i) {
                        self.command(item.2);
                    }
                } else if mode == Popup::Settings && self.dropdown.is_none() && !self.editing {
                    if let Some(i) = self.settings_rects.iter().position(|r| contains(*r)) {
                        self.settings_row = i;
                        self.activate_field();
                    } else if contains(self.popup_rect) && m.row == self.popup_rect.y + 2 {
                        let offset = m.column.saturating_sub(self.popup_rect.x + 2) / 13;
                        self.settings_tab = (offset as usize).min(3);
                        self.settings_row = 0;
                    }
                } else if matches!(mode, Popup::Files | Popup::Find | Popup::Project)
                    && contains(self.result_rect)
                {
                    let height = self.result_rect.height as usize;
                    let start = self.selected.saturating_sub(height.saturating_sub(1) / 2);
                    let i = start + (m.row - self.result_rect.y) as usize;
                    if mode == Popup::Files {
                        if let Some(p) = self.file_hits.get(i).cloned() {
                            self.popup = None;
                            self.open_file(p);
                        }
                    } else if let Some(hit) = self.results.hits.get(i).cloned() {
                        self.selected = i;
                        self.jump_hit(hit);
                    }
                }
            }
            return;
        }
        if contains(self.markdown_rect) {
            if click {
                self.focus = Focus::Editor;
                self.markdown.focused = true;
                if m.row == self.markdown_rect.y && self.markdown_rect.width >= 20 {
                    if m.column >= self.markdown_rect.right() - 4 {
                        self.command(Command::MarkdownToggle);
                    } else if m.column >= self.markdown_rect.right() - 8 {
                        self.command(Command::MarkdownFull);
                    }
                }
            } else if matches!(
                m.kind,
                MouseEventKind::ScrollDown | MouseEventKind::ScrollUp
            ) {
                self.markdown.scroll(
                    if m.kind == MouseEventKind::ScrollDown {
                        3
                    } else {
                        -3
                    },
                    self.markdown_rect.height.saturating_sub(3) as usize,
                );
            }
            return;
        }
        if click {
            if contains(self.editor_rect) {
                self.markdown.focused = false;
            }
            if let Some(i) = self.tab_rects.iter().position(|r| contains(*r)) {
                self.active = i;
                self.markdown.focused = false;
                if self.markdown_mode == MarkdownMode::Preview {
                    self.set_markdown_mode(MarkdownMode::Split);
                }
                self.reveal(Focus::Editor);
                return;
            }
            for (p, r) in [
                (Focus::Tree, self.tree_rect),
                (Focus::Editor, self.editor_rect),
                (Focus::Chat, self.chat_rect),
                (Focus::Terminal, self.term_rect),
            ] {
                if contains(r) {
                    self.focus = p;
                    if m.row == r.y && r.width >= 12 {
                        if m.column >= r.right() - 4 {
                            self.toggle(p);
                            return;
                        }
                        if m.column >= r.right() - 8 {
                            self.command(Command::Zoom);
                            return;
                        }
                    }
                    break;
                }
            }
            if contains(self.tree_rect) && m.row > self.tree_rect.y {
                let i = self.tree.scroll + (m.row - self.tree_rect.y - 1) as usize;
                if i < self.tree.entries.len() {
                    self.tree.selected = i;
                    if let Some(p) = self.tree.toggle_or_open() {
                        self.open_file(p);
                    }
                }
            } else if contains(self.map_rect) && self.has_document && self.map_rect.height > 2 {
                let n = self.doc().textarea.lines().len();
                let bins = (self.map_rect.height - 2) as usize;
                let index = m.row.saturating_sub(self.map_rect.y + 1) as usize;
                let row = if n <= bins {
                    index.min(n - 1)
                } else {
                    index * n / bins
                };
                self.doc_mut()
                    .textarea
                    .move_cursor(CursorMove::Jump(row.min(u16::MAX as usize) as u16, 0));
                self.reveal(Focus::Editor);
            } else if contains(self.editor_rect)
                && self.has_document
                && let Some((row, col)) = self
                    .doc()
                    .textarea
                    .cursor_at_position(Position::new(m.column, m.row))
            {
                self.doc_mut().textarea.cancel_selection();
                self.doc_mut()
                    .textarea
                    .move_cursor(CursorMove::Jump(row as u16, col as u16));
            }
        }
        if matches!(
            m.kind,
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
        ) {
            let down = m.kind == MouseEventKind::ScrollDown;
            if contains(self.editor_rect) {
                self.doc_mut()
                    .textarea
                    .scroll((if down { 3 } else { -3 }, 0));
            } else if contains(self.tree_rect) {
                self.tree.selected = if down {
                    (self.tree.selected + 3).min(self.tree.entries.len().saturating_sub(1))
                } else {
                    self.tree.selected.saturating_sub(3)
                };
            } else if contains(self.chat_rect) {
                self.chat_scroll = if down {
                    self.chat_scroll.saturating_add(3)
                } else {
                    self.chat_scroll.saturating_sub(3)
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> App {
        App::new(
            std::env::current_dir().unwrap(),
            Settings::default(),
            std::env::temp_dir().join("geditor-app-tests/settings.json"),
        )
    }
    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }
    fn chord(a: &mut App, c: char) {
        a.handle_key(ctrl('k'));
        a.handle_key(ctrl(c));
    }
    #[test]
    fn focus_cycles_enabled_panels_and_zoom_restores() {
        let mut a = app();
        a.focus = Focus::Tree;
        for expected in [Focus::Editor, Focus::Chat, Focus::Terminal, Focus::Tree] {
            a.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::CONTROL));
            assert_eq!(a.focus, expected);
        }
        a.show_terminal = false;
        a.show_chat = false;
        a.focus = Focus::Editor;
        a.cycle(false);
        assert_eq!(a.focus, Focus::Tree);
        chord(&mut a, 'z');
        assert_eq!(a.zoom, Some(Focus::Tree));
        a.cycle(false);
        assert_eq!(a.zoom, Some(Focus::Editor));
        chord(&mut a, 'z');
        assert!(a.zoom.is_none());
        assert!(a.show_tree);
        a.show_editor = false;
        a.show_tree = false;
        a.cycle(false);
        assert!(a.show_editor);
    }
    #[test]
    fn legacy_ctrl_chords_and_query_cursor_are_usable() {
        let mut a = app();
        a.handle_key(ctrl('k'));
        a.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(a.popup, Some(Popup::About));
        a.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        let before = a.show_minimap;
        a.handle_key(ctrl('k'));
        a.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(a.show_minimap, !before);
        a.new_file();
        a.paste("ž čar čar");
        a.open_popup(Popup::Find);
        a.paste("čr");
        a.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        a.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        assert_eq!(a.popup_input, "čar");
        assert_eq!(a.results.hits.len(), 2);
        a.handle_key(ctrl('u'));
        assert!(a.popup_input.is_empty());
    }
    #[test]
    fn internal_clipboard_is_shared_between_files() {
        let mut a = app();
        a.paste("čćšđž");
        a.doc_mut().textarea.select_all();
        a.command(Command::Copy);
        a.new_file();
        a.command(Command::Paste);
        assert_eq!(a.doc().text(), "čćšđž");
    }
    #[test]
    fn markdown_preview_modes_update_unsaved_text_and_keep_source_safe() {
        use ratatui::{Terminal, backend::TestBackend};
        let mut a = app();
        a.shell_error = Some("test".into());
        a.new_file();
        a.doc_mut().title = "README.MD".into();
        a.paste("# Old čćšđž\n\n**bold**\n");
        assert!(a.doc().is_markdown());
        let mut terminal = Terminal::new(TestBackend::new(160, 42)).unwrap();
        terminal.draw(|f| crate::ui::draw(f, &mut a)).unwrap();
        assert!(!a.markdown_rect.is_empty());
        assert!(
            a.markdown
                .rows
                .iter()
                .any(|r| r.line.to_string().contains("Old čćšđž"))
        );
        a.doc_mut().textarea.select_all();
        a.paste("# New unsaved\n");
        terminal.draw(|f| crate::ui::draw(f, &mut a)).unwrap();
        assert!(
            a.markdown
                .rows
                .iter()
                .any(|r| r.line.to_string().contains("New unsaved"))
        );
        chord(&mut a, 'l');
        assert_eq!(a.markdown_mode, MarkdownMode::Preview);
        assert_eq!(a.zoom, Some(Focus::Editor));
        terminal.draw(|f| crate::ui::draw(f, &mut a)).unwrap();
        let before = a.doc().text();
        a.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        a.paste("SHOULD NOT INSERT");
        assert_eq!(a.doc().text(), before);
        chord(&mut a, 'l');
        assert_eq!(a.markdown_mode, MarkdownMode::Split);
        assert!(a.zoom.is_none());
        chord(&mut a, 'w');
        assert_eq!(a.markdown_mode, MarkdownMode::Source);
        chord(&mut a, 'w');
        assert_eq!(a.markdown_mode, MarkdownMode::Split);
        for (w, h) in [(28, 8), (52, 13), (80, 24), (160, 42)] {
            let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
            terminal.draw(|f| crate::ui::draw(f, &mut a)).unwrap();
        }
    }
    #[test]
    fn no_file_home_and_close_unsaved_protection() {
        let mut a = app();
        assert!(!a.has_document);
        a.paste("čćšđž\n");
        assert!(a.has_document);
        a.close_file();
        assert!(a.has_document);
        assert_eq!(a.doc().text(), "čćšđž\n");
        a.handle_key(ctrl('q'));
        assert!(a.running);
        a.handle_key(ctrl('q'));
        assert!(!a.running);
    }
    #[test]
    fn find_selects_unicode_exactly_and_can_cancel_invalid_regex() {
        let mut a = app();
        a.paste("ž hello\nlet čar = 42;\n");
        a.open_popup(Popup::Find);
        a.popup_input = "čar".into();
        a.run_search();
        assert_eq!(a.results.hits.len(), 1);
        a.jump_hit(a.results.hits[0].clone());
        assert_eq!(a.doc().textarea.selection_range(), Some(((1, 4), (1, 7))));
        a.open_popup(Popup::Find);
        a.popup_input = "[".into();
        a.search_options.regex = true;
        a.run_search();
        assert!(a.results.error.is_some());
        a.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(a.popup.is_none());
    }
    #[test]
    fn settings_dropdown_cancel_and_portable_section_navigation() {
        let mut a = app();
        chord(&mut a, 'o');
        assert_eq!(a.popup, Some(Popup::Settings));
        a.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(a.dropdown.is_some());
        a.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        a.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(a.settings.theme, "Light");
        a.handle_key(ctrl('n'));
        assert_eq!(a.settings_tab, 1);
        a.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(a.settings.theme, "Midnight");
        assert!(a.popup.is_none());
    }
    #[test]
    fn renders_every_popup_and_theme_in_small_and_large_terminals() {
        use ratatui::{Terminal, backend::TestBackend};
        let mut a = app();
        a.shell_error = Some("test shell disabled".into());
        a.paste("new buffer\nZdravo, čćšđž 世界\n");
        for theme in ["Midnight", "Light", "Terminal"] {
            a.settings.theme = theme.into();
            a.preview_settings();
            a.highlighter.apply(&mut a.docs[0], &a.settings);
            for (w, h) in [(28, 8), (52, 13), (80, 24), (120, 40), (180, 55)] {
                let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
                for popup in [
                    None,
                    Some(Popup::Menu),
                    Some(Popup::Find),
                    Some(Popup::Project),
                    Some(Popup::Files),
                    Some(Popup::Settings),
                    Some(Popup::Help),
                    Some(Popup::About),
                    Some(Popup::SaveAs),
                    Some(Popup::Open),
                ] {
                    a.popup = popup;
                    t.draw(|f| crate::ui::draw(f, &mut a)).unwrap();
                }
            }
        }
    }
    #[test]
    #[ignore = "manual visual review export"]
    fn emit_review_frames_when_requested() {
        let Ok(folder) = std::env::var("GEDITOR_REVIEW_FRAMES") else {
            return;
        };
        use ratatui::{Terminal, backend::TestBackend};
        let mut a = app();
        a.shell_error = Some("Terminal ready — Ctrl+Tab to focus".into());
        std::fs::create_dir_all(&folder).unwrap();
        for name in ["home", "editor", "search", "settings", "about"] {
            a.popup = None;
            if name != "home" && !a.has_document {
                a.new_file();
                a.doc_mut().title = "main.rs".into();
                a.paste("// GEditor — Miodrag Gromilic and OpenAI\nuse std::path::Path;\n\nfn main() {\n    let greeting = \"Zdravo, čćšđž!\";\n    println!(\"{greeting}\");\n}\n");
                a.highlighter.apply(&mut a.docs[0], &a.settings);
            }
            if name == "search" {
                a.open_popup(Popup::Find);
                a.popup_input = "greeting".into();
                a.run_search();
            }
            if name == "settings" {
                a.open_popup(Popup::Settings);
                a.settings_tab = 2;
            }
            if name == "about" {
                a.open_popup(Popup::About);
            }
            let mut t = Terminal::new(TestBackend::new(150, 42)).unwrap();
            t.draw(|f| crate::ui::draw(f, &mut a)).unwrap();
            let b = t.backend().buffer();
            let mut cells = Vec::new();
            for y in 0..42 {
                for x in 0..150 {
                    let c = &b[(x, y)];
                    cells.push(serde_json::json!({"x":x,"y":y,"text":c.symbol(),"fg":format!("{:?}",c.fg),"bg":format!("{:?}",c.bg),"bold":c.modifier.contains(ratatui::style::Modifier::BOLD)}));
                }
            }
            std::fs::write(
                std::path::Path::new(&folder).join(format!("{name}.json")),
                serde_json::to_vec(&cells).unwrap(),
            )
            .unwrap();
        }
    }
}
