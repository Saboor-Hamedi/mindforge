//! Software update settings tab.

use super::SettingPanelAction;
use crate::services::updater::{UpdateManager, UpdateStatus};
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Stroke};

pub fn render_updates_tab(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    panel_rect: Rect,
    p_origin: Pos2,
    updater: &UpdateManager,
    theme: &Theme,
) -> Option<SettingPanelAction> {
    let mut action = None;
    let status = updater.status();
    let card_w = panel_rect.width() - 56.0;

    // ── Header ───────────────────────────────────────────────────────
    painter.text(
        p_origin,
        Align2::LEFT_TOP,
        "SOFTWARE UPDATES",
        FontId::proportional(15.0),
        theme.highlight,
    );
    painter.text(
        p_origin + vec2(0.0, 22.0),
        Align2::LEFT_TOP,
        "Manage updates for MindForge",
        FontId::proportional(12.0),
        theme.muted,
    );

    // Version info card
    let ver_card = Rect::from_min_size(pos2(p_origin.x, p_origin.y + 54.0), vec2(card_w, 64.0));
    painter.rect(
        ver_card,
        6.0,
        theme.surface(),
        Stroke::new(1.0, theme.border()),
        egui::StrokeKind::Inside,
    );
    painter.text(
        ver_card.min + vec2(16.0, 12.0),
        Align2::LEFT_TOP,
        "Current Version",
        FontId::monospace(10.5),
        theme.muted,
    );
    painter.text(
        ver_card.min + vec2(16.0, 30.0),
        Align2::LEFT_TOP,
        &format!("v{}", env!("CARGO_PKG_VERSION")),
        FontId::monospace(13.5),
        theme.accent,
    );

    // Status card
    let status_card =
        Rect::from_min_size(pos2(p_origin.x, p_origin.y + 132.0), vec2(card_w, 120.0));
    painter.rect(
        status_card,
        6.0,
        theme.surface(),
        Stroke::new(1.0, theme.border()),
        egui::StrokeKind::Inside,
    );

    match &status {
        UpdateStatus::Idle => {
            painter.text(
                status_card.min + vec2(16.0, 20.0),
                Align2::LEFT_TOP,
                "⟳  Ready to check for updates",
                FontId::monospace(12.0),
                theme.muted,
            );
        }
        UpdateStatus::Checking => {
            painter.text(
                status_card.min + vec2(16.0, 20.0),
                Align2::LEFT_TOP,
                "⟳  Checking GitHub for latest release...",
                FontId::monospace(12.0),
                theme.accent,
            );
        }
        UpdateStatus::UpToDate { version } => {
            painter.text(
                status_card.min + vec2(16.0, 20.0),
                Align2::LEFT_TOP,
                &format!("✓  You are on the latest version (v{})", version),
                FontId::monospace(12.0),
                theme.accent,
            );
        }
        UpdateStatus::UpdateAvailable {
            new_version,
            release_notes,
            ..
        } => {
            painter.text(
                status_card.min + vec2(16.0, 12.0),
                Align2::LEFT_TOP,
                &format!("🔔  Update available: v{}", new_version),
                FontId::monospace(12.5),
                theme.highlight,
            );
            // Truncate release notes to 2 lines
            let notes: String = release_notes
                .lines()
                .take(2)
                .collect::<Vec<_>>()
                .join(" • ");
            let notes_short = if notes.len() > 80 {
                format!("{}…", &notes[..80])
            } else {
                notes
            };
            painter.text(
                status_card.min + vec2(16.0, 36.0),
                Align2::LEFT_TOP,
                &notes_short,
                FontId::monospace(10.5),
                theme.muted,
            );
        }
        UpdateStatus::Downloading {
            new_version,
            progress,
            downloaded_bytes,
            total_bytes,
        } => {
            painter.text(
                status_card.min + vec2(16.0, 12.0),
                Align2::LEFT_TOP,
                &format!("⬇  Downloading v{}…", new_version),
                FontId::monospace(12.0),
                theme.accent,
            );
            // Progress bar
            let bar_bg = Rect::from_min_size(
                status_card.min + vec2(16.0, 40.0),
                vec2(card_w - 32.0, 10.0),
            );
            painter.rect_filled(bar_bg, 5.0, theme.border());
            if *progress > 0.0 {
                let fill_w = (bar_bg.width() * progress).max(10.0);
                let bar_fill = Rect::from_min_size(bar_bg.min, vec2(fill_w, bar_bg.height()));
                painter.rect_filled(bar_fill, 5.0, theme.accent);
            }
            let mb_done = *downloaded_bytes as f64 / 1_048_576.0;
            let mb_total = *total_bytes as f64 / 1_048_576.0;
            painter.text(
                status_card.min + vec2(16.0, 60.0),
                Align2::LEFT_TOP,
                &format!(
                    "{:.1} MB / {:.1} MB  ({:.0}%)",
                    mb_done,
                    mb_total,
                    progress * 100.0
                ),
                FontId::monospace(10.5),
                theme.muted,
            );
        }
        UpdateStatus::ReadyToRestart { new_version, .. } => {
            painter.text(
                status_card.min + vec2(16.0, 20.0),
                Align2::LEFT_TOP,
                &format!("✓  v{} downloaded — restart to apply", new_version),
                FontId::monospace(12.0),
                theme.accent,
            );
        }
        UpdateStatus::Error(msg) => {
            let short = if msg.len() > 90 {
                format!("{}…", &msg[..90])
            } else {
                msg.clone()
            };
            painter.text(
                status_card.min + vec2(16.0, 16.0),
                Align2::LEFT_TOP,
                &format!("✗  {}", short),
                FontId::monospace(11.0),
                Color32::from_rgb(220, 70, 70),
            );
        }
    }

    // ── Action button — morphs per state ─────────────────────────────
    let (btn_label, can_click) = match &status {
        UpdateStatus::Idle | UpdateStatus::UpToDate { .. } | UpdateStatus::Error(_) => {
            ("⟳  Check for Updates", true)
        }
        UpdateStatus::Checking => ("⟳  Checking…", false),
        UpdateStatus::UpdateAvailable { .. } => ("⬇  Download Update", true),
        UpdateStatus::Downloading { .. } => ("⬇  Downloading…", false),
        UpdateStatus::ReadyToRestart { .. } => ("↺  Restart to Apply Update", true),
    };

    let btn_w = 210.0_f32.min(card_w);
    let btn_h = 32.0;
    let btn_rect = Rect::from_min_size(pos2(p_origin.x, p_origin.y + 266.0), vec2(btn_w, btn_h));
    let btn_resp = ui.allocate_rect(btn_rect, egui::Sense::click());
    let btn_hov = can_click && (btn_resp.hovered() || ui.rect_contains_pointer(btn_rect));
    if btn_hov {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let bg = if btn_hov {
        theme.surface().lerp_to_gamma(theme.accent, 0.05)
    } else {
        theme.surface()
    };
    painter.rect(
        btn_rect,
        5.0,
        bg,
        Stroke::new(
            1.0,
            if btn_hov {
                theme.border().lerp_to_gamma(theme.accent, 0.4)
            } else {
                theme.border()
            },
        ),
        egui::StrokeKind::Inside,
    );
    painter.text(
        btn_rect.center(),
        Align2::CENTER_CENTER,
        btn_label,
        FontId::monospace(12.0),
        if can_click {
            if btn_hov {
                theme.accent
            } else {
                theme.text
            }
        } else {
            theme.muted
        },
    );

    if can_click && (btn_resp.clicked() || (btn_hov && ui.input(|i| i.pointer.primary_clicked()))) {
        let act = match &status {
            UpdateStatus::Idle | UpdateStatus::UpToDate { .. } | UpdateStatus::Error(_) => {
                Some(SettingPanelAction::CheckUpdates)
            }
            UpdateStatus::UpdateAvailable { .. } => Some(SettingPanelAction::DownloadUpdate),
            UpdateStatus::ReadyToRestart { .. } => Some(SettingPanelAction::RestartToApply),
            _ => None,
        };
        if let Some(a) = act {
            action = Some(a);
        }
    }

    action
}
