fn input_with_cursor(input: &str, cursor: usize) -> String {
    let cursor = cursor.min(input.len());
    let mut out = input.to_owned();
    if out.is_char_boundary(cursor) {
        out.insert(cursor, '_');
    }
    out
}
use crate::{
    app::{App, Field, Focus, Popup},
    config::{Palette, Settings},
    search::Hit,
    shell::Shell,
};
use chrono::Local;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Position, Rect},
    style::{Modifier, Style},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

fn block<'a>(label: String, active: bool, s: &Settings, controls: bool) -> Block<'a> {
    let p = s.palette();
    let b = Block::default()
        .borders(Borders::ALL)
        .border_type(if active && !s.ascii {
            BorderType::Rounded
        } else {
            BorderType::Plain
        })
        .title_top(Line::styled(
            format!(" {}{} ", if active { "> " } else { "" }, label),
            Style::default()
                .fg(if active { p.accent } else { p.muted })
                .add_modifier(if active {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ))
        .style(Style::default().fg(p.fg).bg(p.panel))
        .border_style(Style::default().fg(if active { p.accent } else { p.border }));
    let b = if s.ascii {
        b.border_set(border::Set {
            top_left: "+",
            top_right: "+",
            bottom_left: "+",
            bottom_right: "+",
            vertical_left: "|",
            vertical_right: "|",
            horizontal_top: "-",
            horizontal_bottom: "-",
        })
    } else {
        b
    };
    if controls {
        b.title_top(
            Line::styled(" [+] [x] ", Style::default().fg(p.muted)).alignment(Alignment::Right),
        )
    } else {
        b
    }
}
fn line(f: &mut Frame, text: impl Into<Line<'static>>, r: Rect) {
    if !r.is_empty() {
        f.render_widget(Paragraph::new(text.into()), r);
    }
}
fn modal(area: Rect, w: u16, h: u16) -> Rect {
    let width = w.min(area.width.saturating_sub(2)).max(1);
    let height = h.min(area.height.saturating_sub(2)).max(1);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}
pub fn draw(f: &mut Frame, a: &mut App) {
    let area = f.area();
    let p = a.settings.palette();
    f.render_widget(
        Block::default().style(Style::default().bg(p.bg).fg(p.fg)),
        area,
    );
    a.tree_rect = Rect::default();
    a.editor_rect = Rect::default();
    a.chat_rect = Rect::default();
    a.term_rect = Rect::default();
    a.map_rect = Rect::default();
    a.markdown_rect = Rect::default();
    a.tab_rects.clear();
    a.menu_rects.clear();
    if area.width < 28 || area.height < 8 {
        line(
            f,
            "GEditor: resize terminal to at least 28 x 8".to_owned(),
            area,
        );
        return;
    }
    header(f, a, Rect::new(area.x, area.y, area.width, 1));
    tabs(f, a, Rect::new(area.x, area.y + 1, area.width, 1));
    let body = Rect::new(area.x, area.y + 2, area.width, area.height - 4);
    if a.enabled_panels().is_empty() {
        home(f, a, body);
    } else if let Some(focus) = a.zoom {
        draw_panel(f, a, focus, body);
    } else if body.width < 110 || body.height < 16 {
        draw_panel(f, a, a.focus, body);
    } else {
        let parts = if a.show_terminal {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(8),
                    Constraint::Percentage(a.settings.terminal_percent),
                ])
                .split(body)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(0)])
                .split(body)
        };
        let mut constraints = Vec::new();
        let mut panels = Vec::new();
        if a.show_tree {
            panels.push(Focus::Tree);
            constraints.push(if !a.show_editor && !a.show_chat {
                Constraint::Min(1)
            } else {
                Constraint::Length(a.settings.sidebar_width)
            });
        }
        if a.show_editor {
            panels.push(Focus::Editor);
            constraints.push(Constraint::Min(24));
        }
        if a.show_chat {
            panels.push(Focus::Chat);
            constraints.push(if !a.show_editor {
                Constraint::Min(1)
            } else {
                Constraint::Length(a.settings.chat_width)
            });
        }
        if panels.is_empty() {
            if a.show_terminal {
                draw_panel(f, a, Focus::Terminal, body);
            }
        } else {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(constraints)
                .split(parts[0]);
            for (panel, r) in panels.into_iter().zip(cols.iter()) {
                draw_panel(f, a, panel, *r);
            }
            if a.show_terminal {
                draw_panel(f, a, Focus::Terminal, parts[1]);
            }
        }
    }
    status(f, a, Rect::new(area.x, area.bottom() - 2, area.width, 2));
    if let Some(popup) = a.popup {
        popup_draw(f, a, popup, area);
    }
}
fn header(f: &mut Frame, a: &mut App, r: Rect) {
    let p = a.settings.palette();
    f.render_widget(Block::default().style(Style::default().bg(p.panel)), r);
    let mut spans = vec![Span::styled(
        " GEditor ",
        Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
    )];
    let mut x = r.x + 9;
    for (i, name) in ["File", "Edit", "View", "Settings", "Help"]
        .into_iter()
        .enumerate()
    {
        let text = format!(" {name} ");
        let width = text.len() as u16;
        let active = a.popup == Some(Popup::Menu) && a.menu == i;
        spans.push(Span::styled(
            text,
            Style::default()
                .fg(if active { p.bg } else { p.fg })
                .bg(if active { p.accent } else { p.panel }),
        ));
        a.menu_rects
            .push(Rect::new(x, r.y, width.min(r.right().saturating_sub(x)), 1));
        x += width;
    }
    line(f, Line::from(spans), r);
    if r.width > 88 {
        let time = Local::now().format("%d.%m.%Y  %H:%M:%S").to_string();
        line(
            f,
            Line::styled(time, Style::default().fg(p.muted)).alignment(Alignment::Right),
            Rect::new(r.x + 65, r.y, r.width - 66, 1),
        );
    }
}
fn tabs(f: &mut Frame, a: &mut App, r: Rect) {
    let p = a.settings.palette();
    let mut x = r.x;
    let mut spans = Vec::new();
    if a.has_document {
        for (i, d) in a.docs.iter().enumerate() {
            if x >= r.right() {
                break;
            }
            let label = format!(" {}{} ", d.title, if d.is_dirty() { " *" } else { "" });
            let width = UnicodeWidthStr::width(label.as_str()) as u16;
            let selected = i == a.active;
            spans.push(Span::styled(
                label,
                Style::default()
                    .fg(if selected { p.accent } else { p.muted })
                    .bg(if selected { p.selection } else { p.bg })
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ));
            a.tab_rects
                .push(Rect::new(x, r.y, width.min(r.right() - x), 1));
            x = x.saturating_add(width);
        }
    } else {
        spans.push(Span::styled(
            format!(
                " {}  /  Welcome",
                a.root.file_name().unwrap_or_default().to_string_lossy()
            ),
            Style::default().fg(p.muted),
        ));
    }
    line(f, Line::from(spans), r);
}
fn draw_panel(f: &mut Frame, a: &mut App, focus: Focus, r: Rect) {
    if r.width < 3 || r.height < 3 {
        return;
    }
    match focus {
        Focus::Tree => {
            a.tree_rect = r;
            tree(f, a, r)
        }
        Focus::Editor => {
            a.editor_rect = r;
            editor(f, a, r)
        }
        Focus::Chat => {
            a.chat_rect = r;
            chat(f, a, r)
        }
        Focus::Terminal => {
            a.term_rect = r;
            shell(f, a, r)
        }
    }
}
fn tree(f: &mut Frame, a: &mut App, r: Rect) {
    let p = a.settings.palette();
    f.render_widget(
        block(
            "EXPLORER".into(),
            a.focus == Focus::Tree,
            &a.settings,
            r.width > 18,
        ),
        r,
    );
    let inner = r.inner(Margin::new(1, 1));
    let h = inner.height as usize;
    if a.tree.selected < a.tree.scroll {
        a.tree.scroll = a.tree.selected;
    }
    if a.tree.selected >= a.tree.scroll + h {
        a.tree.scroll = a.tree.selected.saturating_sub(h.saturating_sub(1));
    }
    let rows = a
        .tree
        .entries
        .iter()
        .skip(a.tree.scroll)
        .take(h)
        .enumerate()
        .map(|(n, e)| {
            let i = n + a.tree.scroll;
            let expanded = a
                .tree
                .entries
                .get(i + 1)
                .is_some_and(|next| next.depth > e.depth);
            let icon = if e.is_dir {
                if expanded { "- " } else { "+ " }
            } else {
                "  "
            };
            let label = format!(
                "{}{}{}",
                "  ".repeat(e.depth.saturating_sub(1)),
                icon,
                e.path.file_name().unwrap_or_default().to_string_lossy()
            );
            Line::styled(
                label,
                Style::default()
                    .fg(if e.is_dir { p.accent } else { p.fg })
                    .bg(if i == a.tree.selected {
                        p.selection
                    } else {
                        p.panel
                    }),
            )
        })
        .collect::<Vec<_>>();
    f.render_widget(Paragraph::new(rows), inner);
}
fn editor(f: &mut Frame, a: &mut App, r: Rect) {
    use crate::markdown::Mode;
    if !a.has_document || !a.doc().is_markdown() || a.markdown_mode == Mode::Source {
        source_editor(f, a, r);
        return;
    }
    if a.markdown_mode == Mode::Preview {
        a.markdown.focused = true;
        markdown_preview(f, a, r);
    } else {
        let direction = if r.width >= 76 {
            Direction::Horizontal
        } else {
            Direction::Vertical
        };
        let parts = Layout::default()
            .direction(direction)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(r);
        source_editor(f, a, parts[0]);
        markdown_preview(f, a, parts[1]);
    }
}
fn source_editor(f: &mut Frame, a: &mut App, r: Rect) {
    let mut code = r;
    if a.show_minimap && a.has_document && r.width >= 48 {
        let parts = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(30),
                Constraint::Length(a.settings.minimap_width),
            ])
            .split(r);
        code = parts[0];
        a.map_rect = parts[1];
    }
    a.editor_rect = code;
    f.render_widget(
        block(
            if a.has_document {
                format!(
                    "EDITOR  {}{}",
                    a.doc().title,
                    if a.doc().is_dirty() { " *" } else { "" }
                )
            } else {
                "EDITOR".into()
            },
            a.focus == Focus::Editor && !a.markdown.focused,
            &a.settings,
            code.width > 24,
        ),
        code,
    );
    let inner = code.inner(Margin::new(1, 1));
    if a.has_document {
        f.render_widget(&a.doc().textarea, inner);
    } else {
        home(f, a, inner);
    }
    if !a.map_rect.is_empty() {
        minimap(f, a, a.map_rect);
    }
    if a.focus == Focus::Editor
        && !a.markdown.focused
        && a.has_document
        && let Some(pos) = a.doc().textarea.rendered_cursor_position()
    {
        f.set_cursor_position(pos);
    }
}
fn markdown_preview(f: &mut Frame, a: &mut App, r: Rect) {
    if r.width < 3 || r.height < 4 {
        return;
    }
    a.markdown_rect = r;
    let p = a.settings.palette();
    f.render_widget(
        block(
            format!(
                "MARKDOWN  {}{}",
                a.doc().title,
                if a.doc().is_dirty() { " *" } else { "" }
            ),
            a.focus == Focus::Editor && a.markdown.focused,
            &a.settings,
            r.width >= 20,
        ),
        r,
    );
    let inner = r.inner(Margin::new(1, 1));
    let height = inner.height.saturating_sub(1) as usize;
    let identity = format!("{}:{:?}", a.active, a.doc().path);
    a.markdown.update(
        a.docs[a.active].textarea.lines(),
        inner.width as usize,
        &a.settings,
        identity,
    );
    a.markdown.follow(a.doc().textarea.cursor().0, height);
    a.markdown.clamp(height);
    let lines = a
        .markdown
        .rows
        .iter()
        .skip(a.markdown.top)
        .take(height)
        .map(|r| r.line.clone())
        .collect::<Vec<_>>();
    f.render_widget(
        Paragraph::new(lines),
        Rect::new(inner.x, inner.y, inner.width, height as u16),
    );
    line(
        f,
        Line::styled(
            format!(
                "LIVE | {}/{} | ^K ^Y focus | ^K ^L full",
                a.markdown.top + 1,
                a.markdown.rows.len()
            ),
            Style::default().fg(p.muted),
        ),
        Rect::new(inner.x, inner.bottom() - 1, inner.width, 1),
    );
}
fn home(f: &mut Frame, a: &App, r: Rect) {
    if r.is_empty() {
        return;
    }
    let p = a.settings.palette();
    let now = Local::now();
    let time = now.format("%H:%M:%S").to_string();
    let mut rows = vec![
        Line::styled(
            "GEditor",
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
        ),
        Line::raw(""),
    ];
    if a.settings.clock_large && r.width >= 36 && r.height >= 14 {
        let digits = [
            ["###", "# #", "# #", "# #", "###"],
            [" # ", "## ", " # ", " # ", "###"],
            ["###", "  #", "###", "#  ", "###"],
            ["###", "  #", " ##", "  #", "###"],
            ["# #", "# #", "###", "  #", "  #"],
            ["###", "#  ", "###", "  #", "###"],
            ["###", "#  ", "###", "# #", "###"],
            ["###", "  #", "  #", "  #", "  #"],
            ["###", "# #", "###", "# #", "###"],
            ["###", "# #", "###", "  #", "###"],
        ];
        for (y, _) in digits[0].iter().enumerate() {
            let mut text = String::new();
            for c in time.chars() {
                let glyph = if c == ':' {
                    if y == 1 || y == 3 { " # " } else { "   " }
                } else {
                    digits[c.to_digit(10).unwrap() as usize][y]
                };
                text.push_str(&glyph.replace('#', if a.settings.ascii { "#" } else { "█" }));
                text.push(' ');
            }
            rows.push(Line::styled(text, Style::default().fg(p.accent)));
        }
    } else {
        rows.push(Line::styled(
            time,
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
        ));
    }
    rows.push(Line::raw(""));
    rows.push(Line::styled(
        now.format("%A  %d.%m.%Y").to_string(),
        Style::default().fg(p.fg),
    ));
    rows.push(Line::raw(""));
    rows.push(Line::styled(
        "Ctrl+N new   Ctrl+P open   Ctrl+E menu",
        Style::default().fg(p.muted),
    ));
    if r.height >= 18 {
        rows.push(Line::styled(
            "Ctrl+Tab panels   Ctrl+K Ctrl+Z maximize",
            Style::default().fg(p.muted),
        ));
    }
    let height = (rows.len() as u16).min(r.height);
    let target = Rect::new(r.x, r.y + (r.height - height) / 2, r.width, height);
    f.render_widget(Paragraph::new(rows).alignment(Alignment::Center), target);
}
fn minimap(f: &mut Frame, a: &App, r: Rect) {
    let p = a.settings.palette();
    f.render_widget(block("MAP".into(), false, &a.settings, false), r);
    let inside = r.inner(Margin::new(1, 1));
    let h = inside.height as usize;
    let w = inside.width.saturating_sub(1) as usize;
    if h == 0 || w == 0 {
        return;
    }
    let lines = a.doc().textarea.lines();
    let n = lines.len();
    let editor = a.editor_rect.inner(Margin::new(1, 1));
    let first = a
        .doc()
        .textarea
        .cursor_at_position(Position::new(editor.x, editor.y))
        .map(|p| p.0)
        .unwrap_or(0);
    let last = a
        .doc()
        .textarea
        .cursor_at_position(Position::new(editor.x, editor.bottom().saturating_sub(1)))
        .map(|p| p.0)
        .unwrap_or(first);
    let cursor = a.doc().textarea.cursor().0;
    let mut rows = Vec::new();
    for y in 0..h {
        if n <= h && y >= n {
            rows.push(Line::raw(""));
            continue;
        }
        let start = if n <= h { y } else { y * n / h };
        let end = if n <= h {
            y + 1
        } else {
            ((y + 1) * n / h).max(start + 1)
        }
        .min(n);
        let visible = start <= last && end > first;
        let current = cursor >= start && cursor < end;
        let mut counts = vec![0usize; w];
        let samples = [start, (start + end - 1) / 2, end - 1];
        for row in samples {
            for (i, c) in lines[row].chars().take(w * 4).enumerate() {
                if !c.is_whitespace() {
                    counts[i / 4] += 1;
                }
            }
        }
        let glyphs = if a.settings.ascii {
            [' ', '.', ':', '#']
        } else {
            [' ', '░', '▒', '▓']
        };
        let text: String = counts
            .into_iter()
            .map(|n| {
                glyphs[if n == 0 {
                    0
                } else if n < 4 {
                    1
                } else if n < 8 {
                    2
                } else {
                    3
                }]
            })
            .collect();
        let indicator = if current {
            ">"
        } else if visible {
            "|"
        } else {
            " "
        };
        rows.push(Line::from(vec![
            Span::styled(
                indicator,
                Style::default().fg(if current { p.accent } else { p.muted }),
            ),
            Span::styled(
                text,
                Style::default()
                    .fg(if current { p.accent } else { p.muted })
                    .bg(if visible { p.selection } else { p.panel }),
            ),
        ]));
    }
    f.render_widget(Paragraph::new(rows), inside);
}
fn chat(f: &mut Frame, a: &mut App, r: Rect) {
    let p = a.settings.palette();
    f.render_widget(
        block(
            format!(
                "AI{}",
                if a.chat_rx.is_some() {
                    " / thinking"
                } else {
                    " CHAT"
                }
            ),
            a.focus == Focus::Chat,
            &a.settings,
            r.width > 18,
        ),
        r,
    );
    let inner = r.inner(Margin::new(1, 1));
    if inner.height < 4 {
        return;
    }
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(4),
        ])
        .split(inner);
    line(
        f,
        Line::styled(
            format!(
                "{} | context {}",
                a.settings.ai_model,
                if a.settings.ai_context { "on" } else { "off" }
            ),
            Style::default().fg(p.muted),
        ),
        parts[0],
    );
    let mut rows = Vec::new();
    for m in &a.chat {
        rows.push(Line::styled(
            if m.role == "user" { "YOU" } else { "ASSISTANT" },
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
        ));
        for row in wrap_text(&m.content, parts[1].width as usize) {
            rows.push(Line::styled(row, Style::default().fg(p.fg)));
        }
        rows.push(Line::raw(""));
    }
    let max = rows.len().saturating_sub(parts[1].height as usize);
    a.chat_scroll = a.chat_scroll.min(max);
    f.render_widget(
        Paragraph::new(rows).scroll((a.chat_scroll.min(u16::MAX as usize) as u16, 0)),
        parts[1],
    );
    let value = if a.chat_input.is_empty() {
        "Ask about your code..."
    } else {
        &a.chat_input
    };
    f.render_widget(
        Paragraph::new(value)
            .wrap(Wrap { trim: false })
            .block(block(
                "Enter sends / paste for multiline".into(),
                a.focus == Focus::Chat,
                &a.settings,
                false,
            ))
            .style(Style::default().fg(if a.chat_input.is_empty() {
                p.muted
            } else {
                p.fg
            })),
        parts[2],
    );
}
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut rows = Vec::new();
    for source in text.lines() {
        let mut row = String::new();
        let mut count = 0;
        for c in source.chars() {
            let w = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            if count + w > width && !row.is_empty() {
                rows.push(std::mem::take(&mut row));
                count = 0;
            }
            row.push(c);
            count += w;
        }
        rows.push(row);
    }
    rows
}
fn shell(f: &mut Frame, a: &mut App, r: Rect) {
    let p = a.settings.palette();
    f.render_widget(
        block(
            "TERMINAL".into(),
            a.focus == Focus::Terminal,
            &a.settings,
            r.width > 18,
        ),
        r,
    );
    let inside = r.inner(Margin::new(1, 1));
    a.ensure_shell();
    if let Some(shell) = a.shell.as_mut() {
        shell.resize(inside.width, inside.height);
        let screen = shell.parser.screen();
        for y in 0..inside.height {
            for x in 0..inside.width {
                if let Some(c) = screen.cell(y, x) {
                    if c.is_wide_continuation() {
                        continue;
                    }
                    if let Some(dst) = f.buffer_mut().cell_mut((inside.x + x, inside.y + y)) {
                        let mut style = Shell::cell_style(c);
                        style.fg = style.fg.map(|c| a.settings.color(c));
                        style.bg = style.bg.map(|c| a.settings.color(c));
                        dst.set_symbol(if c.has_contents() { c.contents() } else { " " })
                            .set_style(style);
                    }
                }
            }
        }
        if a.focus == Focus::Terminal && !screen.hide_cursor() {
            let (y, x) = screen.cursor_position();
            if y < inside.height && x < inside.width {
                f.set_cursor_position((inside.x + x, inside.y + y));
            }
        }
    } else {
        f.render_widget(
            Paragraph::new(a.shell_error.as_deref().unwrap_or("Terminal unavailable"))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(p.danger)),
            inside,
        );
    }
}
fn status(f: &mut Frame, a: &App, r: Rect) {
    let p = a.settings.palette();
    let location = if a.has_document {
        let (row, col) = a.doc().textarea.cursor();
        format!(
            "  {}:{}  {}{}",
            row + 1,
            col + 1,
            if a.doc().is_dirty() {
                "modified"
            } else {
                "saved"
            },
            if a.zoom.is_some() { " / MAX" } else { "" }
        )
    } else {
        String::new()
    };
    line(
        f,
        Line::from(vec![
            Span::styled(
                format!(" {} ", a.focus.name()),
                Style::default()
                    .fg(p.bg)
                    .bg(p.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}{}", a.status, location),
                Style::default().fg(p.fg),
            ),
        ]),
        Rect::new(r.x, r.y, r.width, 1),
    );
    let panes = a
        .enabled_panels()
        .into_iter()
        .map(|p| format!("{}{}", if p == a.focus { ">" } else { "" }, p.name()))
        .collect::<Vec<_>>()
        .join(" / ");
    let footer = if a.prefix {
        "Ctrl+K command pending | press another Ctrl key | Esc cancels".into()
    } else {
        format!(" Ctrl+Tab panels | Ctrl+E menu | Ctrl+K Ctrl+H help   {panes}")
    };
    line(
        f,
        Line::styled(footer, Style::default().fg(p.muted)),
        Rect::new(r.x, r.y + 1, r.width, 1),
    );
}
fn popup_draw(f: &mut Frame, a: &mut App, mode: Popup, area: Rect) {
    match mode {
        Popup::Menu => menu(f, a, area),
        Popup::Settings => settings(f, a, area),
        Popup::Find | Popup::Project | Popup::Files => search_popup(f, a, mode, area),
        Popup::Help | Popup::About => info(f, a, mode, area),
        _ => input_popup(f, a, mode, area),
    }
}
fn menu(f: &mut Frame, a: &mut App, area: Rect) {
    let items = a.menu_items();
    let width = 61.min(area.width.saturating_sub(2));
    let x = a
        .menu_rects
        .get(a.menu)
        .map(|r| r.x)
        .unwrap_or(1)
        .min(area.right().saturating_sub(width));
    let r = Rect::new(
        x,
        area.y + 1,
        width,
        (items.len() as u16 + 4).min(area.height.saturating_sub(2)),
    );
    a.popup_rect = r;
    f.render_widget(Clear, r);
    f.render_widget(
        block(
            ["File", "Edit", "View", "Settings", "Help"][a.menu].into(),
            true,
            &a.settings,
            false,
        ),
        r,
    );
    let p = a.settings.palette();
    let inner = r.inner(Margin::new(1, 1));
    a.result_rect = Rect::new(
        inner.x,
        inner.y,
        inner.width,
        inner.height.saturating_sub(1),
    );
    let rows = items
        .into_iter()
        .enumerate()
        .take(a.result_rect.height as usize)
        .map(|(i, (label, hint, _))| {
            let width = inner.width as usize;
            let spaces =
                width.saturating_sub(UnicodeWidthStr::width(label.as_str()) + hint.len() + 3);
            Line::styled(
                format!(" {}{}{} ", label, " ".repeat(spaces), hint),
                Style::default()
                    .fg(if i == a.menu_row { p.accent } else { p.fg })
                    .bg(if i == a.menu_row {
                        p.selection
                    } else {
                        p.panel
                    }),
            )
        })
        .collect::<Vec<_>>();
    f.render_widget(Paragraph::new(rows), a.result_rect);
    line(
        f,
        Line::styled(
            " Left/Right menu | Up/Down | Enter | Esc",
            Style::default().fg(p.muted),
        ),
        Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1),
    );
}
fn input_popup(f: &mut Frame, a: &mut App, mode: Popup, area: Rect) {
    let r = modal(area, 88, 9);
    a.popup_rect = r;
    let p = a.settings.palette();
    f.render_widget(Clear, r);
    f.render_widget(
        block(
            if mode == Popup::SaveAs {
                "Save As / existing files are protected"
            } else {
                "Open file or directory"
            }
            .into(),
            true,
            &a.settings,
            false,
        ),
        r,
    );
    let inner = r.inner(Margin::new(2, 1));
    line(
        f,
        Line::styled(
            format!("> {}", input_with_cursor(&a.popup_input, a.input_cursor)),
            Style::default().fg(p.accent),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    f.render_widget(Paragraph::new(format!("Relative to {}\nAbsolute paths and ~/ are supported.\nEnter confirms | Esc cancels\n{}",a.root.display(),a.status)).wrap(Wrap{trim:false}).style(Style::default().fg(p.muted)),Rect::new(inner.x,inner.y+2,inner.width,inner.height.saturating_sub(2)));
}
fn search_popup(f: &mut Frame, a: &mut App, mode: Popup, area: Rect) {
    let r = modal(
        area,
        area.width.saturating_sub(6).min(150),
        area.height.saturating_sub(4).min(34),
    );
    a.popup_rect = r;
    let p = a.settings.palette();
    f.render_widget(Clear, r);
    f.render_widget(
        block(
            match mode {
                Popup::Files => "QUICK OPEN / files",
                Popup::Find => "FIND / current buffer",
                _ => "SEARCH / entire workspace",
            }
            .into(),
            true,
            &a.settings,
            false,
        ),
        r,
    );
    let inner = r.inner(Margin::new(2, 1));
    if inner.height < 5 {
        return;
    }
    line(
        f,
        Line::styled(
            format!(
                "{} {}",
                if a.search_glob_edit { "Query:" } else { ">" },
                input_with_cursor(&a.popup_input, a.input_cursor)
            ),
            Style::default().fg(if a.search_glob_edit { p.fg } else { p.accent }),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let is_search = mode != Popup::Files;
    let mut y = inner.y + 2;
    if is_search {
        let o = &a.search_options;
        line(
            f,
            Line::styled(
                format!(
                    "[{}] Case ^C  [{}] Word ^W  [{}] Regex ^R  [{}] Hidden ^D  [{}] Ignored ^A",
                    if o.case { "x" } else { " " },
                    if o.word { "x" } else { " " },
                    if o.regex { "x" } else { " " },
                    if o.hidden { "x" } else { " " },
                    if o.ignored { "x" } else { " " }
                ),
                Style::default().fg(p.muted),
            ),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y += 1;
        line(
            f,
            Line::styled(
                format!(
                    "{}Files: {}   (Ctrl+T edit filter; e.g. *.rs,*.toml)",
                    if a.search_glob_edit { "> " } else { "  " },
                    if o.glob.is_empty() { "all" } else { &o.glob }
                ),
                Style::default().fg(if a.search_glob_edit {
                    p.accent
                } else {
                    p.muted
                }),
            ),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y += 1;
    }
    let summary = if mode == Popup::Find && !a.has_document {
        "No file open. Esc, then Ctrl+P opens a file.".into()
    } else if mode == Popup::Files {
        if a.index_job.is_some() {
            "Indexing files...".into()
        } else {
            format!(
                "{} matching files | .gitignore respected",
                a.file_hits.len()
            )
        }
    } else if let Some(e) = &a.results.error {
        format!("ERROR: {e}")
    } else if a.search_job.is_some() {
        "Searching in background... keep typing to cancel".into()
    } else if a.search_pending {
        "Enter starts search | Ctrl+S reruns".into()
    } else {
        format!(
            "{} matches / {} files scanned / {} skipped{}",
            a.results.hits.len(),
            a.results.scanned,
            a.results.skipped,
            if a.results.truncated {
                " / LIMIT REACHED"
            } else {
                ""
            }
        )
    };
    line(
        f,
        Line::styled(
            summary,
            Style::default().fg(if a.results.error.is_some() {
                p.danger
            } else {
                p.accent
            }),
        ),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 1;
    let bottom = inner.bottom().saturating_sub(2);
    let height = bottom.saturating_sub(y);
    a.result_rect = Rect::new(inner.x, y, inner.width, height);
    let start = a
        .selected
        .saturating_sub((height as usize).saturating_sub(1) / 2);
    let rows = if mode == Popup::Files {
        a.file_hits
            .iter()
            .skip(start)
            .take(height as usize)
            .enumerate()
            .map(|(i, path)| {
                let selected = i + start == a.selected;
                Line::styled(
                    format!(
                        "{} {}",
                        if selected { ">" } else { " " },
                        path.strip_prefix(&a.root).unwrap_or(path).display()
                    ),
                    Style::default()
                        .fg(if selected { p.accent } else { p.fg })
                        .bg(if selected { p.selection } else { p.panel }),
                )
            })
            .collect::<Vec<_>>()
    } else {
        let re = search_regex(a);
        a.results
            .hits
            .iter()
            .skip(start)
            .take(height as usize)
            .enumerate()
            .map(|(i, hit)| hit_line(a, hit, start + i == a.selected, &p, re.as_ref()))
            .collect::<Vec<_>>()
    };
    f.render_widget(Paragraph::new(rows), a.result_rect);
    line(
        f,
        Line::styled(
            "Up/Down or click selects | Enter opens exact match | Esc closes",
            Style::default().fg(p.muted),
        ),
        Rect::new(inner.x, bottom, inner.width, 1),
    );
    if is_search {
        line(
            f,
            Line::styled(
                "Line-based; open buffers included. Skips binaries and >2 MiB. ^A includes build folders.",
                Style::default().fg(p.muted),
            ),
            Rect::new(inner.x, bottom + 1, inner.width, 1),
        );
    }
}
fn search_regex(a: &App) -> Option<regex::Regex> {
    crate::search::compile(&a.popup_input, &a.search_options).ok()
}
fn hit_line(
    a: &App,
    hit: &Hit,
    selected: bool,
    p: &Palette,
    re: Option<&regex::Regex>,
) -> Line<'static> {
    let prefix = format!(
        "{} {}:{}:{}  ",
        if selected { ">" } else { " " },
        hit.path
            .strip_prefix(&a.root)
            .unwrap_or(&hit.path)
            .display(),
        hit.row + 1,
        hit.col + 1
    );
    let mut spans = vec![Span::styled(
        prefix,
        Style::default().fg(if selected { p.accent } else { p.muted }),
    )];
    let mut start = 0;
    if let Some(re) = re {
        for found in re.find_iter(&hit.preview) {
            spans.push(Span::styled(
                hit.preview[start..found.start()].to_owned(),
                Style::default().fg(p.fg),
            ));
            spans.push(Span::styled(
                found.as_str().to_owned(),
                Style::default()
                    .fg(p.accent)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ));
            start = found.end();
        }
    }
    spans.push(Span::styled(
        hit.preview[start..].to_owned(),
        Style::default().fg(p.fg),
    ));
    Line::from(spans).style(Style::default().bg(if selected { p.selection } else { p.panel }))
}
fn settings(f: &mut Frame, a: &mut App, area: Rect) {
    let r = modal(area, 96, 27);
    a.popup_rect = r;
    let p = a.settings.palette();
    f.render_widget(Clear, r);
    f.render_widget(
        block(
            "SETTINGS / live preview / Ctrl+S saves".into(),
            true,
            &a.settings,
            false,
        ),
        r,
    );
    let inner = r.inner(Margin::new(2, 1));
    if inner.height < 7 {
        return;
    }
    let tabs = ["Appearance", "Panels", "AI", "General"]
        .into_iter()
        .enumerate()
        .map(|(i, t)| {
            Span::styled(
                format!(" {:<12}", t),
                Style::default()
                    .fg(if i == a.settings_tab {
                        p.accent
                    } else {
                        p.muted
                    })
                    .add_modifier(if i == a.settings_tab {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            )
        })
        .collect::<Vec<_>>();
    line(
        f,
        Line::from(tabs),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    let rows_start = inner.y + 3;
    let available = inner.height.saturating_sub(9) as usize;
    let start = a.settings_row.saturating_sub(available.saturating_sub(1));
    a.settings_rects = vec![Rect::default(); a.fields().len()];
    for (i, field) in a.fields().iter().enumerate().skip(start).take(available) {
        let (label, value, _) = a.field_row(*field);
        let selected = i == a.settings_row;
        let text = if selected && a.editing {
            if *field == Field::ApiKey {
                format!("{}_", "*".repeat(a.edit_text.chars().count().min(40)))
            } else {
                format!("{}_", a.edit_text)
            }
        } else {
            value
        };
        let row = Rect::new(inner.x, rows_start + (i - start) as u16, inner.width, 1);
        a.settings_rects[i] = row;
        let label_width = 24usize.min(inner.width as usize / 2);
        let spans = vec![
            Span::styled(
                format!(
                    "{} {:<width$}",
                    if selected { ">" } else { " " },
                    label,
                    width = label_width
                ),
                Style::default().fg(if selected { p.accent } else { p.fg }),
            ),
            Span::styled(text, Style::default().fg(p.fg)),
        ];
        line(
            f,
            Line::from(spans).style(Style::default().bg(if selected {
                p.selection
            } else {
                p.panel
            })),
            row,
        );
    }
    let hint = a.field_row(a.fields()[a.settings_row]).2;
    f.render_widget(
        Paragraph::new(hint)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(p.muted)),
        Rect::new(inner.x, inner.bottom() - 5, inner.width, 2),
    );
    line(
        f,
        Line::styled(
            if a.editing {
                "Enter accepts field | Ctrl+U clears | Esc cancels field"
            } else {
                "Up/Down field | Enter edit/dropdown | Left/Right change | Ctrl+Tab or Ctrl+N/P section"
            },
            Style::default().fg(p.muted),
        ),
        Rect::new(inner.x, inner.bottom() - 3, inner.width, 1),
    );
    line(
        f,
        Line::styled(
            "Ctrl+S save and apply | Esc discard | Keys are stored locally with mode 0600",
            Style::default().fg(p.accent),
        ),
        Rect::new(inner.x, inner.bottom() - 2, inner.width, 1),
    );
    line(
        f,
        Line::styled(a.status.clone(), Style::default().fg(p.muted)),
        Rect::new(inner.x, inner.bottom() - 1, inner.width, 1),
    );
    if let Some(drop) = &a.dropdown {
        let dr = modal(
            r,
            r.width.saturating_sub(12).min(72),
            (drop.items.len() as u16 + 4).min(r.height.saturating_sub(4)),
        );
        f.render_widget(Clear, dr);
        f.render_widget(
            block(
                "Choose / Enter confirms / Esc cancels".into(),
                true,
                &a.settings,
                false,
            ),
            dr,
        );
        let di = dr.inner(Margin::new(1, 1));
        a.dropdown_rect = di;
        let start = drop
            .selected
            .saturating_sub((di.height as usize).saturating_sub(1) / 2);
        let rows = drop
            .items
            .iter()
            .enumerate()
            .skip(start)
            .take(di.height as usize)
            .map(|(i, item)| {
                Line::styled(
                    format!("{} {}", if i == drop.selected { ">" } else { " " }, item),
                    Style::default()
                        .fg(if i == drop.selected { p.accent } else { p.fg })
                        .bg(if i == drop.selected {
                            p.selection
                        } else {
                            p.panel
                        }),
                )
            })
            .collect::<Vec<_>>();
        f.render_widget(Paragraph::new(rows), di);
    }
}
const HELP: &str = "GEditor keyboard commands\n\nCtrl+Tab          Next enabled panel\nCtrl+Shift+Tab    Previous enabled panel\nCtrl+K Ctrl+F     Portable next-panel fallback\nCtrl+E            Menu (outside shell)\nCtrl+K Ctrl+V     Menu (everywhere)\nCtrl+N / Ctrl+O   New file / open file or folder\nCtrl+P            Quick Open indexed project files\nCtrl+S / Ctrl+W   Save / close file\nCtrl+F / Ctrl+G   Find in file / search workspace\nCtrl+U / Ctrl+R   Undo / redo\nCtrl+A/C/X/V      Select all / copy / cut / internal paste\nCtrl+Q            Quit; repeat to discard unsaved changes\n\nPress Ctrl+K, then the second Ctrl combination:\nCtrl+B / Ctrl+E   Toggle explorer / editor\nCtrl+A / Ctrl+T   Toggle AI / terminal\nCtrl+M            Toggle minimap\nCtrl+W            Markdown preview on/off\nCtrl+L            Full Markdown preview / split\nCtrl+Y            Focus Markdown preview / source\nCtrl+Z            Maximize / restore active panel\nCtrl+R            Restore saved default layout\nCtrl+C            Hide active panel\nCtrl+S            Save As\nCtrl+D / Ctrl+U   Next / previous file tab\nCtrl+N / Ctrl+P   Next / previous search match\nCtrl+O            Settings\nCtrl+H / Ctrl+I   Help / About\nCtrl+K            Send Ctrl+K to child shell\n\nSearch popup: Ctrl+C case, Ctrl+W whole word, Ctrl+R regex,\nCtrl+D hidden, Ctrl+A ignored/build, Ctrl+T filter, Ctrl+S run, Ctrl+U clear.\nResults show file:line:column; Enter/click selects exact code.\nSettings: Ctrl+Tab or Ctrl+N/P switches sections; Enter opens dropdowns,\nCtrl+S saves, Esc discards. Keys are masked and stored locally.\nIn the shell, Ctrl+C/D/L/E and other editing keys go to zsh.\nUse Ctrl+Tab or Ctrl+K commands to navigate out of the shell.\nSome terminals intercept Ctrl+Tab; use Ctrl+K Ctrl+F instead.\nFont and font size belong to your terminal (Mac: Cmd +/-).\nNarrow screens show the focused panel; Ctrl+Tab reveals others.\n\nEsc closes this window.";
fn info(f: &mut Frame, a: &mut App, mode: Popup, area: Rect) {
    let p = a.settings.palette();
    let r = modal(
        area,
        90,
        if mode == Popup::Help {
            area.height.saturating_sub(2)
        } else {
            20
        },
    );
    a.popup_rect = r;
    f.render_widget(Clear, r);
    f.render_widget(
        block(
            if mode == Popup::Help {
                "HELP / commands / Up Down scroll"
            } else {
                "ABOUT GEditor"
            }
            .into(),
            true,
            &a.settings,
            false,
        ),
        r,
    );
    let content = if mode == Popup::Help {
        HELP.to_owned()
    } else {
        format!(
            "GEditor {}\n\nFree terminal code editor\n\nAuthors: {}\nCreated with assistance from OpenAI Codex.\n\nWritten in Rust with Ratatui and Crossterm.\nLicense: MIT OR Apache-2.0 — free to use, modify and redistribute.\nAI provider usage may have separate charges.\n\nSettings: {}\n\nEnter or Esc closes this window.",
            env!("CARGO_PKG_VERSION"),
            a.settings.author,
            a.settings_path.display()
        )
    };
    f.render_widget(
        Paragraph::new(content)
            .wrap(Wrap { trim: false })
            .scroll((a.selected.min(u16::MAX as usize) as u16, 0))
            .style(Style::default().fg(p.fg)),
        r.inner(Margin::new(2, 1)),
    );
}
