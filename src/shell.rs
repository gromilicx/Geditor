use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use ratatui::style::{Color, Modifier, Style};
use std::{
    env,
    io::{Read, Write},
    num::NonZeroU16,
    path::Path,
    sync::mpsc::{self, Receiver},
    thread,
};

pub struct Shell {
    _child: Box<dyn Child + Send>,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    rx: Receiver<Vec<u8>>,
    pub parser: vt100::Parser,
    pub cols: u16,
    pub rows: u16,
}

impl Shell {
    pub fn new(root: &Path) -> Result<Self> {
        let pty = native_pty_system();
        let cols = 80u16;
        let rows = 12u16;
        let pair = pty
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Opening pseudo-terminal")?;
        let default_shell = if cfg!(windows) {
            env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
        } else {
            env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
        };
        let mut cmd = CommandBuilder::new(default_shell);
        cmd.cwd(root.as_os_str());
        cmd.env("TERM", "xterm-256color");
        let child = pair.slave.spawn_command(cmd).context("Starting shell")?;
        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        drop(pair.slave);
        // Backpressure keeps a noisy child from filling memory while hidden.
        let (tx, rx) = mpsc::sync_channel(32);
        thread::spawn(move || {
            let mut data = [0u8; 8192];
            loop {
                match reader.read(&mut data) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(data[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        Ok(Self {
            _child: child,
            master: pair.master,
            writer,
            rx,
            parser: vt100::Parser::new(
                NonZeroU16::new(rows).unwrap(),
                NonZeroU16::new(cols).unwrap(),
                3000,
            ),
            cols,
            rows,
        })
    }

    pub fn tick(&mut self) {
        // Yield to keyboard input even when a command produces output forever.
        for _ in 0..16 {
            let Ok(bytes) = self.rx.try_recv() else { break };
            self.parser.process(&bytes);
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let cols = cols.max(2);
        let rows = rows.max(2);
        if cols == self.cols && rows == self.rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
        self.parser.set_size(
            NonZeroU16::new(rows).unwrap(),
            NonZeroU16::new(cols).unwrap(),
        );
    }

    pub fn paste(&mut self, text: &str) -> Result<()> {
        if self.parser.screen().bracketed_paste() {
            self.writer.write_all(b"\x1b[200~")?;
            self.writer.write_all(text.as_bytes())?;
            self.writer.write_all(b"\x1b[201~")?;
        } else {
            self.writer
                .write_all(text.replace("\r\n", "\n").replace('\n', "\r").as_bytes())?;
        }
        self.writer.flush()?;
        Ok(())
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let seq: Vec<u8> = match key.code {
            KeyCode::Char(ch) if ctrl && ch.is_ascii_alphabetic() => {
                vec![(ch.to_ascii_lowercase() as u8) - b'a' + 1]
            }
            KeyCode::Char(ch) => {
                let mut seq = Vec::new();
                if alt {
                    seq.push(0x1b);
                }
                let mut b = [0; 4];
                seq.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
                seq
            }
            KeyCode::Enter => b"\r".to_vec(),
            KeyCode::Tab => b"\t".to_vec(),
            KeyCode::Backspace => vec![0x7f],
            KeyCode::Esc => vec![0x1b],
            KeyCode::Up => b"\x1b[A".to_vec(),
            KeyCode::Down => b"\x1b[B".to_vec(),
            KeyCode::Right => b"\x1b[C".to_vec(),
            KeyCode::Left => b"\x1b[D".to_vec(),
            KeyCode::Home => b"\x1b[H".to_vec(),
            KeyCode::End => b"\x1b[F".to_vec(),
            KeyCode::Delete => b"\x1b[3~".to_vec(),
            KeyCode::PageUp => b"\x1b[5~".to_vec(),
            KeyCode::PageDown => b"\x1b[6~".to_vec(),
            _ => return Ok(()),
        };
        self.writer.write_all(&seq)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn cell_style(cell: &vt100::Cell) -> Style {
        let fg = match cell.fgcolor() {
            vt100::Color::Default => Color::Rgb(207, 220, 234),
            c => Self::ansi_color(c),
        };
        let bg = match cell.bgcolor() {
            vt100::Color::Default => Color::Rgb(23, 28, 40),
            c => Self::ansi_color(c),
        };
        let mut style = Style::default().fg(fg).bg(bg);
        if cell.bold() {
            style = style.add_modifier(Modifier::BOLD);
        }
        if cell.underline() {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        if cell.italic() {
            style = style.add_modifier(Modifier::ITALIC);
        }
        if cell.inverse() {
            style = style.add_modifier(Modifier::REVERSED);
        }
        style
    }

    fn ansi_color(c: vt100::Color) -> Color {
        match c {
            vt100::Color::Default => Color::Rgb(207, 220, 234),
            vt100::Color::Idx(i) => Color::Indexed(i),
            vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
        }
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        let _ = self._child.kill();
        let _ = self._child.wait();
    }
}
