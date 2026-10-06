//! Appearance, themes, audio profiles, fonts, and carets (:theme, :sound, :caret, :font, :opacity).

use crate::app::App;
use crate::caret::CaretKind;
use crate::services::db_worker::DbMsg;
use crate::services::sound::SoundProfile;
use crate::ui::theme::{Theme, ThemeKind};

pub fn handle(app: &mut App, cmd: &str, args: &str, _raw: &str, now: f64) -> bool {
    match cmd {
        "opacity" => {
            let clean = args.trim();
            if let Ok(v) = clean.parse::<f32>() {
                let op = if v > 1.0 { v / 100.0 } else { v }.clamp(0.2, 1.0);
                app.misc.opacity = op;
                let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                    key: "opacity".into(),
                    val: format!("{:.2}", op),
                });
                app.set_status(format!("Window opacity set to {:.0}%", op * 100.0), now);
            } else {
                app.set_status(
                    format!(
                        "Current opacity: {:.0}% (:opacity 0.20 - 1.00)",
                        app.misc.opacity * 100.0
                    ),
                    now,
                );
            }
            true
        }
        "font" | "fonts" => {
            let target = args.trim();
            if target.is_empty() {
                app.modal.settings_open = true;
                app.modal.settings_just_opened = true;
                app.modal.settings_opened_at = now;
                app.modal.active_setting_tab = crate::settings::SettingTab::Fonts;
                app.set_status("Font preferences opened", now);
            } else {
                app.misc.selected_font = target.to_string();
                let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                    key: "selected_font".into(),
                    val: target.to_string(),
                });
                app.editor.font_dirty = true;
                app.set_status(format!("Editor font set to {}", target), now);
            }
            true
        }
        "sound" | "audio" => {
            if let Some(profile) = SoundProfile::parse(args) {
                app.misc.sound.profile = profile;
                let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                    key: "sound".into(),
                    val: profile.name().to_lowercase(),
                });
                app.set_status(format!("Typing sound set to: {}", profile.name()), now);
            } else {
                app.set_status(
                    "Usage: :sound <off|thocky|clacky|creamy|marbly|poppy|clicky>",
                    now,
                );
            }
            true
        }
        "volume" | "mute" | "unmute" => {
            if cmd == "mute" {
                app.misc.sound.profile = SoundProfile::Off;
                let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                    key: "sound".into(),
                    val: "off".into(),
                });
                app.set_status("Sound muted (:sound off)", now);
            } else if cmd == "unmute" {
                app.misc.sound.profile = SoundProfile::Thocky;
                let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                    key: "sound".into(),
                    val: "thocky".into(),
                });
                app.set_status("Sound unmuted (Thocky profile)", now);
            }
            true
        }
        "caret" | "cursor" => {
            if let Some(kind) = CaretKind::parse(args) {
                app.misc.caret.kind = kind;
                let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                    key: "caret".into(),
                    val: args.into(),
                });
                app.set_status(format!("Caret set to {}", args), now);
            } else {
                app.set_status("Usage: :caret <block|beam|underline|candle|fire|water|snow|neon|rainbow|electric|comet|matrix|ice|glitch|heartbeat>", now);
            }
            true
        }
        "theme" | "colorscheme" | "color" => {
            if let Some(kind) = ThemeKind::parse(args) {
                app.misc.theme = Theme::from_kind(kind);
                app.misc.accent_overrides.apply(&mut app.misc.theme);
                if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                    backend.sync_theme(&app.misc.theme);
                }
                let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                    key: "theme".into(),
                    val: args.into(),
                });
                app.set_status(format!("Theme set to {}", args), now);
            } else {
                app.set_status("Usage: :theme <tokyonight|catppuccin|dracula|gruvbox|nord|monokai|rosepine|solarized|onedark|ayu>", now);
            }
            true
        }
        _ => false,
    }
}
