//! Safe, local CommonMark/GFM rendering for the terminal; no HTML execution or resource fetching.
use crate::config::Settings;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const MAX_ROWS: usize = 20_000;
#[derive(Clone)]
pub struct Row {
    pub line: Line<'static>,
    pub source: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Source,
    Split,
    Preview,
}
#[derive(Default)]
pub struct Cache {
    pub rows: Vec<Row>,
    pub top: usize,
    pub focused: bool,
    source: String,
    width: usize,
    theme: String,
    identity: String,
}
impl Cache {
    pub fn update(
        &mut self,
        lines: &[String],
        width: usize,
        settings: &Settings,
        identity: String,
    ) {
        let theme = format!(
            "{}|{}|{}|{}|{}|{}|{}",
            settings.theme,
            settings.color_mode,
            settings.accent,
            settings.background,
            settings.foreground,
            settings.ascii,
            settings.tab_width
        );
        let switched = self.identity != identity;
        if switched
            || width != self.width
            || theme != self.theme
            || !self.source.split('\n').eq(lines.iter().map(String::as_str))
        {
            self.source = lines.join("\n");
            self.rows = render(&self.source, width, settings);
            self.width = width;
            self.theme = theme;
            self.identity = identity;
            if switched {
                self.top = 0;
            }
        }
    }
    pub fn follow(&mut self, source: usize, height: usize) {
        if self.focused || height == 0 {
            return;
        }
        let target = self
            .rows
            .iter()
            .position(|r| r.source >= source)
            .unwrap_or_else(|| self.rows.len().saturating_sub(1));
        if target < self.top {
            self.top = target;
        } else if target >= self.top + height {
            self.top = target.saturating_sub(height / 2);
        }
        self.clamp(height);
    }
    pub fn clamp(&mut self, height: usize) {
        self.top = self.top.min(self.rows.len().saturating_sub(height));
    }
    pub fn scroll(&mut self, delta: isize, height: usize) {
        self.focused = true;
        self.top = self.top.saturating_add_signed(delta);
        self.clamp(height);
    }
}
fn clean(text: &str) -> String {
    text.chars()
        .filter_map(|c| {
            if c == '\t' {
                Some(' ')
            } else if c == '\n' || !c.is_control() {
                Some(c)
            } else {
                None
            }
        })
        .collect()
}
fn append(spans: &mut Vec<Span<'static>>, text: String, style: Style) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = spans.last_mut()
        && last.style == style
    {
        last.content.to_mut().push_str(&text);
    } else {
        spans.push(Span::styled(text, style));
    }
}
fn wrap(
    spans: Vec<Span<'static>>,
    width: usize,
    prefixes: (&str, &str),
    source: usize,
    rows: &mut Vec<Row>,
    base: Style,
    preserve: bool,
) {
    let (prefix, continuation) = prefixes;
    let width = width.max(1);
    let mut line = vec![Span::styled(prefix.to_owned(), base)];
    let mut used = UnicodeWidthStr::width(prefix).min(width);
    let mut content = false;
    for span in spans {
        for word in span.content.split_inclusive(char::is_whitespace) {
            let word_width = UnicodeWidthStr::width(word.trim_end());
            if content
                && used + word_width > width
                && word_width <= width.saturating_sub(UnicodeWidthStr::width(continuation))
            {
                rows.push(Row {
                    line: Line::from(std::mem::take(&mut line)),
                    source,
                });
                if rows.len() >= MAX_ROWS {
                    return;
                }
                line.push(Span::styled(continuation.to_owned(), base));
                used = UnicodeWidthStr::width(continuation);
                content = false;
            }
            for c in word.chars() {
                if c == '\n' {
                    continue;
                }
                let w = c.width().unwrap_or(0);
                if !preserve && !content && c.is_whitespace() {
                    continue;
                }
                if used + w > width && content {
                    rows.push(Row {
                        line: Line::from(std::mem::take(&mut line)),
                        source,
                    });
                    if rows.len() >= MAX_ROWS {
                        return;
                    }
                    line.push(Span::styled(continuation.to_owned(), base));
                    used = UnicodeWidthStr::width(continuation);
                }
                append(&mut line, c.to_string(), span.style);
                used += w;
                content = true;
            }
        }
    }
    rows.push(Row {
        line: Line::from(line),
        source,
    });
}
struct Builder<'a> {
    settings: &'a Settings,
    width: usize,
    rows: Vec<Row>,
    current: Vec<Span<'static>>,
    source: usize,
    styles: Vec<Style>,
    lists: Vec<Option<u64>>,
    quotes: usize,
    item_prefix: Option<String>,
    code: bool,
    table: Option<Table>,
    links: Vec<String>,
}
struct Table {
    rows: Vec<(Vec<Vec<Span<'static>>>, usize, bool)>,
    cells: Vec<Vec<Span<'static>>>,
    header: bool,
}
impl<'a> Builder<'a> {
    fn style(&self) -> Style {
        *self.styles.last().unwrap()
    }
    fn push(&mut self, text: &str) {
        let style = self.style();
        let text = if self.code {
            text.replace('\t', &" ".repeat(self.settings.tab_width as usize))
        } else {
            text.to_owned()
        };
        append(&mut self.current, clean(&text), style);
    }
    fn prefix(&self) -> String {
        let quote = if self.settings.ascii { "> " } else { "│ " };
        format!(
            "{}{}",
            quote.repeat(self.quotes),
            self.item_prefix.as_deref().unwrap_or("")
        )
    }
    fn flush(&mut self) {
        if self.current.is_empty() {
            return;
        }
        let p = self.prefix();
        let continuation = format!(
            "{}{}",
            if self.settings.ascii { "> " } else { "│ " }.repeat(self.quotes),
            " ".repeat(
                self.item_prefix
                    .as_deref()
                    .map(UnicodeWidthStr::width)
                    .unwrap_or(0)
            )
        );
        let spans = std::mem::take(&mut self.current);
        let style = self.style();
        wrap(
            spans,
            self.width,
            (&p, &continuation),
            self.source,
            &mut self.rows,
            style,
            self.code,
        );
    }
    fn blank(&mut self) {
        self.flush();
        if self.rows.last().is_some_and(|r| r.line.width() > 0) {
            self.rows.push(Row {
                line: Line::raw(""),
                source: self.source,
            });
        }
    }
    fn table_end(&mut self) {
        let Some(table) = self.table.take() else {
            return;
        };
        let cols = table
            .rows
            .iter()
            .map(|(cells, _, _)| cells.len())
            .max()
            .unwrap_or(0)
            .min(32);
        if cols == 0 {
            return;
        }
        let cell_width = self
            .width
            .saturating_sub(cols + 1)
            .checked_div(cols)
            .unwrap_or(1)
            .max(1);
        let p = self.settings.palette();
        let separator = format!(
            "+{}+",
            std::iter::repeat_n("-".repeat(cell_width), cols)
                .collect::<Vec<_>>()
                .join("+")
        );
        self.rows.push(Row {
            line: Line::styled(separator.clone(), Style::default().fg(p.border)),
            source: self.source,
        });
        for (cells, source, header) in table.rows {
            let mut wrapped = Vec::new();
            for cell in cells.into_iter().take(cols) {
                let mut rows = Vec::new();
                wrap(
                    cell,
                    cell_width,
                    ("", ""),
                    source,
                    &mut rows,
                    Style::default().fg(p.fg),
                    false,
                );
                wrapped.push(rows);
            }
            let count = wrapped.iter().map(Vec::len).max().unwrap_or(1);
            for y in 0..count {
                let mut spans = vec![Span::styled("|", Style::default().fg(p.border))];
                for cell in &wrapped {
                    let line = cell.get(y).map(|r| r.line.clone()).unwrap_or_default();
                    let used = line.width();
                    for mut span in line.spans {
                        if header {
                            span.style = span.style.fg(p.accent).add_modifier(Modifier::BOLD);
                        }
                        spans.push(span);
                    }
                    spans.push(Span::raw(" ".repeat(cell_width.saturating_sub(used))));
                    spans.push(Span::styled("|", Style::default().fg(p.border)));
                }
                self.rows.push(Row {
                    line: Line::from(spans),
                    source,
                });
                if self.rows.len() >= MAX_ROWS {
                    return;
                }
            }
            if header {
                self.rows.push(Row {
                    line: Line::styled(separator.clone(), Style::default().fg(p.border)),
                    source,
                });
            }
        }
        self.rows.push(Row {
            line: Line::styled(separator, Style::default().fg(p.border)),
            source: self.source,
        });
        self.blank();
    }
}
pub fn render(source: &str, width: usize, settings: &Settings) -> Vec<Row> {
    let p = settings.palette();
    let base = Style::default().fg(p.fg).bg(p.panel);
    let mut b = Builder {
        settings,
        width: width.max(1),
        rows: Vec::new(),
        current: Vec::new(),
        source: 0,
        styles: vec![base],
        lists: Vec::new(),
        quotes: 0,
        item_prefix: None,
        code: false,
        table: None,
        links: Vec::new(),
    };
    let starts: Vec<_> = std::iter::once(0)
        .chain(source.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_FOOTNOTES;
    for (event, range) in Parser::new_ext(source, options).into_offset_iter() {
        if b.rows.len() >= MAX_ROWS {
            break;
        }
        let source_row = starts
            .partition_point(|&n| n <= range.start)
            .saturating_sub(1);
        if b.current.is_empty() {
            b.source = source_row;
        }
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {}
                Tag::Heading { level, .. } => {
                    b.blank();
                    b.styles.push(base.fg(p.accent).add_modifier(
                        Modifier::BOLD
                            | if level as u8 <= 2 {
                                Modifier::UNDERLINED
                            } else {
                                Modifier::empty()
                            },
                    ));
                }
                Tag::Emphasis => b.styles.push(b.style().add_modifier(Modifier::ITALIC)),
                Tag::Strong => b.styles.push(b.style().add_modifier(Modifier::BOLD)),
                Tag::Strikethrough => b.styles.push(b.style().add_modifier(Modifier::CROSSED_OUT)),
                Tag::BlockQuote(_) => {
                    b.blank();
                    b.quotes += 1;
                }
                Tag::List(start) => {
                    b.flush();
                    b.lists.push(start);
                }
                Tag::Item => {
                    b.flush();
                    let depth = b.lists.len().saturating_sub(1);
                    let marker = if let Some(Some(n)) = b.lists.last_mut() {
                        let v = format!("{n}. ");
                        *n += 1;
                        v
                    } else {
                        format!("{} ", if settings.ascii { "-" } else { "•" })
                    };
                    b.item_prefix = Some(format!("{}{marker}", "  ".repeat(depth)));
                }
                Tag::CodeBlock(kind) => {
                    b.blank();
                    let language = match kind {
                        CodeBlockKind::Fenced(s) => s.into_string(),
                        _ => String::new(),
                    };
                    b.current.push(Span::styled(
                        format!(
                            "[code{}]",
                            if language.is_empty() {
                                String::new()
                            } else {
                                format!(": {language}")
                            }
                        ),
                        base.fg(p.muted),
                    ));
                    b.flush();
                    b.code = true;
                    b.styles.push(base.bg(p.selection));
                }
                Tag::Link { dest_url, .. } => {
                    b.links.push(dest_url.into_string());
                    b.styles
                        .push(b.style().fg(p.accent).add_modifier(Modifier::UNDERLINED));
                }
                Tag::Image { dest_url, .. } => {
                    b.links.push(dest_url.into_string());
                    b.push("[Image: ");
                }
                Tag::FootnoteDefinition(label) => {
                    b.blank();
                    b.push(&format!("[{label}] "));
                }
                Tag::Table(_) => {
                    b.blank();
                    b.table = Some(Table {
                        rows: Vec::new(),
                        cells: Vec::new(),
                        header: false,
                    });
                }
                Tag::TableHead => {
                    if let Some(t) = &mut b.table {
                        t.header = true;
                    }
                }
                Tag::TableRow => {}
                Tag::TableCell => {
                    b.current.clear();
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => {
                    if b.lists.is_empty() {
                        b.blank();
                    } else {
                        b.flush();
                    }
                }
                TagEnd::Heading(_) => {
                    b.flush();
                    b.styles.pop();
                    b.blank();
                }
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => {
                    b.styles.pop();
                }
                TagEnd::BlockQuote(_) => {
                    b.flush();
                    b.quotes = b.quotes.saturating_sub(1);
                    b.blank();
                }
                TagEnd::List(_) => {
                    b.flush();
                    b.lists.pop();
                    b.item_prefix = None;
                    if b.lists.is_empty() {
                        b.blank();
                    }
                }
                TagEnd::Item => {
                    b.flush();
                    b.item_prefix = None;
                }
                TagEnd::CodeBlock => {
                    b.flush();
                    b.code = false;
                    b.styles.pop();
                    b.blank();
                }
                TagEnd::Link => {
                    b.styles.pop();
                    if let Some(url) = b.links.pop() {
                        b.push(&format!(" ({url})"));
                    }
                }
                TagEnd::Image => {
                    b.push("]");
                    if let Some(url) = b.links.pop() {
                        b.push(&format!(" ({url})"));
                    }
                }
                TagEnd::FootnoteDefinition => b.blank(),
                TagEnd::TableCell => {
                    if let Some(t) = &mut b.table {
                        t.cells.push(std::mem::take(&mut b.current));
                    }
                }
                TagEnd::TableHead | TagEnd::TableRow => {
                    if let Some(t) = &mut b.table {
                        let cells = std::mem::take(&mut t.cells);
                        t.rows.push((cells, source_row, t.header));
                        t.header = false;
                    }
                }
                TagEnd::Table => b.table_end(),
                _ => {}
            },
            Event::Text(text) if b.code => {
                for (i, row) in text.split_terminator('\n').enumerate() {
                    b.source = source_row + i;
                    b.push(row);
                    if b.current.is_empty() {
                        b.current.push(Span::styled(" ", b.style()));
                    }
                    b.flush();
                    if b.rows.len() >= MAX_ROWS {
                        break;
                    }
                }
            }
            Event::Text(text) => b.push(&text),
            Event::Code(text) => {
                b.current.push(Span::styled(
                    clean(&text),
                    b.style().fg(p.accent).bg(p.selection),
                ));
            }
            Event::SoftBreak => b.push(" "),
            Event::HardBreak => b.flush(),
            Event::Rule => {
                b.blank();
                b.push(&if settings.ascii { "-" } else { "─" }.repeat(width.clamp(1, 80)));
                b.blank();
            }
            Event::TaskListMarker(done) => b.push(if done { "[x] " } else { "[ ] " }),
            Event::FootnoteReference(label) => b.push(&format!("[{label}]")),
            Event::Html(text) | Event::InlineHtml(text) => {
                b.push(&text);
                if text.ends_with('\n') {
                    b.flush();
                }
            }
            Event::InlineMath(text) | Event::DisplayMath(text) => b.push(&text),
        }
    }
    b.flush();
    while b.rows.last().is_some_and(|r| r.line.width() == 0) {
        b.rows.pop();
    }
    if b.rows.len() >= MAX_ROWS {
        b.rows.truncate(MAX_ROWS);
        b.rows.push(Row {
            line: Line::styled(
                "Preview limit reached (20,000 rows); full source is still editable.",
                base.fg(p.danger),
            ),
            source: starts.len() - 1,
        });
    }
    if b.rows.is_empty() {
        b.rows.push(Row {
            line: Line::styled("Empty Markdown document", base.fg(p.muted)),
            source: 0,
        });
    }
    b.rows
}
#[cfg(test)]
mod tests {
    use super::*;
    fn text(rows: &[Row]) -> String {
        rows.iter()
            .map(|r| r.line.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
    #[test]
    fn renders_commonmark_gfm_unicode_and_styles() {
        let md = "# Naslov čćšđž\n\n**bold** and *italic* with `code`\n\n- [x] Done\n- [ ] Todo\n\n> quote\n\n```rust\n    let ž = 42;\n```\n\n| A | B |\n|---|---|\n| č | 2 |\n\n[site](https://example.com)\n![alt](image.png)\n";
        let rows = render(md, 60, &Settings::default());
        let out = text(&rows);
        for expected in [
            "Naslov čćšđž",
            "bold and italic",
            "[x] Done",
            "[ ] Todo",
            "quote",
            "    let ž = 42;",
            "https://example.com",
            "Image: alt",
            "image.png",
            "|č",
        ] {
            assert!(out.contains(expected), "Missing {expected} in {out}");
        }
        assert!(!out.contains("**bold**"));
        assert!(rows.iter().any(|r| {
            r.line
                .spans
                .iter()
                .any(|s| s.style.add_modifier.contains(Modifier::BOLD))
        }));
        assert!(rows.iter().all(|r| r.line.width() <= 60));
    }
    #[test]
    fn cache_refreshes_unsaved_text_and_width_and_strips_controls() {
        let s = Settings::default();
        let mut c = Cache::default();
        c.update(&["# Old".into()], 30, &s, "doc".into());
        c.update(&["# New čćšđž".into()], 12, &s, "doc".into());
        assert!(text(&c.rows).contains("New čćšđž"));
        assert!(c.rows.iter().all(|r| r.line.width() <= 12));
        assert!(!text(&render("x\x1b[31m y\x07", 20, &s)).contains(['\x1b', '\x07']));
    }
}
