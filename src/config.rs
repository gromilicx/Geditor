use anyhow::{Context, Result, bail};
use ratatui::style::{Color, Style};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyProfile {
    pub name: String,
    pub key: String,
}
impl Default for KeyProfile {
    fn default() -> Self {
        Self {
            name: "Personal".into(),
            key: String::new(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    pub color_mode: String,
    pub accent: String,
    pub background: String,
    pub foreground: String,
    pub tree: bool,
    pub editor: bool,
    pub chat: bool,
    pub terminal: bool,
    pub minimap: bool,
    pub markdown_preview: bool,
    pub sidebar_width: u16,
    pub chat_width: u16,
    pub terminal_percent: u16,
    pub minimap_width: u16,
    pub tab_width: u8,
    pub line_numbers: bool,
    pub ascii: bool,
    pub clock_large: bool,
    pub ai_model: String,
    pub ai_models: Vec<String>,
    pub ai_base: String,
    pub ai_profile: usize,
    pub keys: Vec<KeyProfile>,
    pub ai_context: bool,
    pub author: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "Midnight".into(),
            color_mode: "Auto".into(),
            accent: String::new(),
            background: String::new(),
            foreground: String::new(),
            tree: true,
            editor: true,
            chat: true,
            terminal: true,
            minimap: true,
            markdown_preview: true,
            sidebar_width: 24,
            chat_width: 36,
            terminal_percent: 28,
            minimap_width: 9,
            tab_width: 4,
            line_numbers: true,
            ascii: false,
            clock_large: true,
            ai_model: env::var("GEDITOR_AI_MODEL")
                .or_else(|_| env::var("FORGEDIT_AI_MODEL"))
                .unwrap_or_else(|_| "gpt-4.1-mini".into()),
            ai_models: vec!["gpt-4.1-mini".into()],
            ai_base: "https://api.openai.com/v1".into(),
            ai_profile: 0,
            keys: vec![
                KeyProfile::default(),
                KeyProfile {
                    name: "Work".into(),
                    key: String::new(),
                },
                KeyProfile {
                    name: "Other".into(),
                    key: String::new(),
                },
            ],
            ai_context: true,
            author: "Miodrag Gromilic and OpenAI".into(),
        }
    }
}
impl Settings {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let mut s: Self = serde_json::from_slice(&fs::read(path)?)
            .context("Invalid settings JSON; original file left untouched")?;
        s.sidebar_width = s.sidebar_width.clamp(16, 50);
        s.chat_width = s.chat_width.clamp(24, 80);
        s.terminal_percent = s.terminal_percent.clamp(15, 65);
        s.minimap_width = s.minimap_width.clamp(5, 20);
        s.tab_width = s.tab_width.clamp(1, 8);
        if s.keys.is_empty() {
            s.keys.push(KeyProfile::default());
        }
        s.ai_profile = s.ai_profile.min(s.keys.len());
        if s.ai_models.is_empty() {
            s.ai_models.push(s.ai_model.clone());
        }
        Ok(s)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        let parent = path.parent().context("Settings directory missing")?;
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
        let temp = parent.join(format!(".settings-{}.tmp", std::process::id()));
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let result = (|| -> Result<()> {
            let mut file = opts.open(&temp)?;
            file.write_all(&serde_json::to_vec_pretty(self)?)?;
            file.sync_all()?;
            fs::rename(&temp, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }
    pub fn validate(&self) -> Result<()> {
        for value in [&self.accent, &self.background, &self.foreground] {
            if !value.is_empty() && parse_hex(value).is_none() {
                bail!("Colors must be #RRGGBB or empty for theme default")
            }
        }
        let url = reqwest::Url::parse(&self.ai_base).context("AI base URL is invalid")?;
        if url.scheme() != "https"
            && !(url.scheme() == "http"
                && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")))
        {
            bail!("Use HTTPS for AI, or HTTP on localhost")
        }
        if self.ai_model.trim().is_empty() {
            bail!("Model ID cannot be empty")
        }
        Ok(())
    }
    pub fn api_key(&self) -> Result<String, String> {
        if self.ai_profile == 0 {
            env::var("OPENAI_API_KEY")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| "Choose a key profile in Settings, or set OPENAI_API_KEY".into())
        } else {
            self.keys
                .get(self.ai_profile - 1)
                .map(|p| p.key.clone())
                .filter(|k| !k.trim().is_empty())
                .ok_or_else(|| {
                    "Selected key profile is empty. Open Settings to add its key.".into()
                })
        }
    }
    pub fn palette(&self) -> Palette {
        let (bg, panel, fg, muted, border, accent, selection) = match self.theme.as_str() {
            "Light" => (
                0xf2f4f8, 0xffffff, 0x172033, 0x536179, 0xa1aec2, 0x155bc0, 0xd6e6ff,
            ),
            "Dracula" => (
                0x20212b, 0x282a36, 0xf8f8f2, 0xa6b0d0, 0x596282, 0xbd93f9, 0x44475a,
            ),
            _ => (
                0x0e1420, 0x151e2d, 0xe2eaf5, 0x9caec6, 0x42516a, 0x72cbb0, 0x273c52,
            ),
        };
        let rgb = |v| Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
        let cv = |v| self.color(rgb(v));
        let terminal = self.theme == "Terminal";
        Palette {
            bg: if terminal {
                Color::Reset
            } else {
                self.color(parse_hex(&self.background).unwrap_or(rgb(bg)))
            },
            panel: if terminal {
                Color::Reset
            } else if !self.background.is_empty() {
                self.color(parse_hex(&self.background).unwrap_or(rgb(panel)))
            } else {
                cv(panel)
            },
            fg: if terminal {
                Color::Reset
            } else {
                self.color(parse_hex(&self.foreground).unwrap_or(rgb(fg)))
            },
            muted: if terminal { Color::Gray } else { cv(muted) },
            border: cv(border),
            accent: self.color(parse_hex(&self.accent).unwrap_or(rgb(accent))),
            selection: cv(selection),
            danger: cv(0xf1848e),
        }
    }
    pub fn color(&self, color: Color) -> Color {
        let mode = if self.color_mode == "Auto" {
            if env::var_os("NO_COLOR").is_some() {
                "Mono"
            } else if env::var("COLORTERM")
                .is_ok_and(|v| v.contains("truecolor") || v.contains("24bit"))
            {
                "True color"
            } else if env::var("TERM").is_ok_and(|v| v.contains("256color")) {
                "256 colors"
            } else {
                "16 colors"
            }
        } else {
            &self.color_mode
        };
        let Color::Rgb(r, g, b) = color else {
            return color;
        };
        match mode {
            "Mono" => Color::Reset,
            "16 colors" => {
                let colors = [
                    (0, 0, 0),
                    (170, 0, 0),
                    (0, 170, 0),
                    (170, 85, 0),
                    (0, 0, 170),
                    (170, 0, 170),
                    (0, 170, 170),
                    (170, 170, 170),
                    (85, 85, 85),
                    (255, 85, 85),
                    (85, 255, 85),
                    (255, 255, 85),
                    (85, 85, 255),
                    (255, 85, 255),
                    (85, 255, 255),
                    (255, 255, 255),
                ];
                let (idx, _) = colors
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, (cr, cg, cb))| distance((r, g, b), (*cr, *cg, *cb)))
                    .unwrap();
                Color::Indexed(idx as u8)
            }
            "256 colors" => {
                let mut best = (0, i32::MAX);
                for idx in 16..=255u16 {
                    let rgb = if idx < 232 {
                        let n = idx - 16;
                        let v = [0, 95, 135, 175, 215, 255];
                        (
                            v[(n / 36) as usize],
                            v[((n / 6) % 6) as usize],
                            v[(n % 6) as usize],
                        )
                    } else {
                        let c = (8 + 10 * (idx - 232)) as u8;
                        (c, c, c)
                    };
                    let d = distance((r, g, b), rgb);
                    if d < best.1 {
                        best = (idx as u8, d);
                    }
                }
                Color::Indexed(best.0)
            }
            _ => color,
        }
    }
    pub fn style_doc(&self, doc: &mut crate::document::Document) {
        let p = self.palette();
        doc.textarea
            .set_style(Style::default().fg(p.fg).bg(p.panel));
        doc.textarea
            .set_cursor_style(Style::default().fg(p.bg).bg(p.accent));
        doc.textarea
            .set_cursor_line_style(Style::default().bg(p.selection));
        doc.textarea.set_selection_style(
            Style::default()
                .bg(p.selection)
                .add_modifier(ratatui::style::Modifier::UNDERLINED),
        );
        doc.textarea.set_search_style(
            Style::default()
                .bg(p.selection)
                .add_modifier(ratatui::style::Modifier::BOLD),
        );
        if self.line_numbers {
            doc.textarea
                .set_line_number_style(Style::default().fg(p.muted));
        } else {
            doc.textarea.remove_line_number();
        }
        doc.textarea.set_tab_length(self.tab_width);
        doc.highlight_dirty = true;
    }
}
fn distance(a: (u8, u8, u8), b: (u8, u8, u8)) -> i32 {
    (a.0 as i32 - b.0 as i32).pow(2)
        + (a.1 as i32 - b.1 as i32).pow(2)
        + (a.2 as i32 - b.2 as i32).pow(2)
}
pub fn parse_hex(s: &str) -> Option<Color> {
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() != 6 {
        return None;
    }
    let n = u32::from_str_radix(s, 16).ok()?;
    Some(Color::Rgb((n >> 16) as u8, (n >> 8) as u8, n as u8))
}
pub struct Palette {
    pub bg: Color,
    pub panel: Color,
    pub fg: Color,
    pub muted: Color,
    pub border: Color,
    pub accent: Color,
    pub selection: Color,
    pub danger: Color,
}
pub fn path() -> PathBuf {
    config_path_with(|key| env::var_os(key), cfg!(windows))
}
fn config_path_with(get: impl Fn(&str) -> Option<std::ffi::OsString>, windows: bool) -> PathBuf {
    if let Some(p) = get("GEDITOR_CONFIG_DIR") {
        return PathBuf::from(p).join("settings.json");
    }
    if let Some(p) = get("XDG_CONFIG_HOME") {
        return PathBuf::from(p).join("geditor/settings.json");
    }
    if windows {
        let base = get("APPDATA").map(PathBuf::from).unwrap_or_else(|| {
            PathBuf::from(
                get("USERPROFILE")
                    .or_else(|| get("HOME"))
                    .unwrap_or_default(),
            )
            .join("AppData/Roaming")
        });
        base.join("GEditor/settings.json")
    } else {
        PathBuf::from(get("HOME").unwrap_or_default()).join(".config/geditor/settings.json")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_settings_use_appdata_and_respect_overrides() {
        let get = |key: &str| match key {
            "APPDATA" => Some(std::ffi::OsString::from("profile/roaming")),
            "HOME" => Some(std::ffi::OsString::from("home")),
            _ => None,
        };
        assert_eq!(
            config_path_with(get, true),
            PathBuf::from("profile/roaming/GEditor/settings.json")
        );
        assert_eq!(
            config_path_with(get, false),
            PathBuf::from("home/.config/geditor/settings.json")
        );
        assert_eq!(
            config_path_with(
                |key| if key == "GEDITOR_CONFIG_DIR" {
                    Some("custom".into())
                } else {
                    get(key)
                },
                true
            ),
            PathBuf::from("custom/settings.json")
        );
    }
    #[test]
    fn settings_round_trip_and_permissions() {
        let path = env::temp_dir()
            .join(format!("geditor-config-{}", std::process::id()))
            .join("settings.json");
        let s = Settings {
            theme: "Light".into(),
            tree: false,
            ai_profile: 2,
            ..Settings::default()
        };
        s.save(&path).unwrap();
        let loaded = Settings::load(&path).unwrap();
        assert_eq!(loaded.theme, "Light");
        assert!(!loaded.tree);
        assert_eq!(loaded.ai_profile, 2);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
