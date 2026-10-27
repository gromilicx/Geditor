use crate::document::Document;
use ratatui::style::{Color, Style};
use std::path::Path;
use syntect::{
    easy::HighlightLines,
    highlighting::{Theme, ThemeSet},
    parsing::SyntaxSet,
};

pub struct Highlighter {
    syntax: SyntaxSet,
    theme: Theme,
    light_theme: Theme,
}

impl Highlighter {
    pub fn new() -> Self {
        let syntax = SyntaxSet::load_defaults_newlines();
        let themes = ThemeSet::load_defaults();
        let theme = themes
            .themes
            .get("base16-ocean.dark")
            .or_else(|| themes.themes.values().next())
            .expect("syntect default themes")
            .clone();
        let light_theme = themes
            .themes
            .get("InspiredGitHub")
            .unwrap_or(&theme)
            .clone();
        Self {
            syntax,
            theme,
            light_theme,
        }
    }

    pub fn apply(&self, doc: &mut Document, settings: &crate::config::Settings) {
        if !doc.highlight_dirty {
            return;
        }
        doc.highlight_dirty = false;
        doc.textarea.clear_custom_highlight();
        let extension = doc.path.as_deref().unwrap_or(Path::new(&doc.title));
        let syntax = self
            .syntax
            .find_syntax_for_file(extension)
            .ok()
            .flatten()
            .unwrap_or_else(|| self.syntax.find_syntax_plain_text());
        let theme = if settings.theme == "Light" {
            &self.light_theme
        } else {
            &self.theme
        };
        let mut styles = std::collections::HashMap::new();
        let mut highlighter = HighlightLines::new(syntax, theme);
        // Protect responsiveness on large files. Syntax grammars remain stateful across lines.
        // Increase MAX_HIGHLIGHT_LINES if editing very large files.
        for row in 0..doc.textarea.lines().len().min(3_000) {
            let original_line = doc.textarea.lines()[row].clone();
            let with_eol = format!("{original_line}\n");
            if let Ok(ranges) = highlighter.highlight_line(&with_eol, &self.syntax) {
                let mut col = 0;
                for (style, chunk) in ranges {
                    // Custom highlight ranges are UTF-8 byte offsets, unlike cursor columns.
                    let count = chunk.trim_end_matches('\n').len();
                    if count > 0 {
                        let color = style.foreground;
                        doc.textarea.custom_highlight(
                            ((row, col), (row, col + count)),
                            *styles
                                .entry((color.r, color.g, color.b))
                                .or_insert_with(|| {
                                    Style::default()
                                        .fg(settings.color(Color::Rgb(color.r, color.g, color.b)))
                                }),
                            1,
                        );
                        col += count;
                    }
                }
            }
        }
    }
}
