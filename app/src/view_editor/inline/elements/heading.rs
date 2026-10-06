//! Heading styling colors for preview.

use crate::ui::theme::Theme;
use eframe::egui::Color32;

/// Returns the primary text color for a given heading level.
pub fn heading_color(level: u8, theme: &Theme) -> Color32 {
    let is_light = theme.is_light();
    match level {
        1 => theme.accent,
        2 => {
            if is_light {
                Color32::from_rgb(217, 119, 6)
            } else {
                Color32::from_rgb(229, 192, 123)
            }
        }
        3 => {
            if is_light {
                Color32::from_rgb(2, 132, 199)
            } else {
                Color32::from_rgb(97, 175, 239)
            }
        }
        4 => {
            if is_light {
                Color32::from_rgb(22, 163, 74)
            } else {
                Color32::from_rgb(152, 195, 121)
            }
        }
        5 => {
            if is_light {
                Color32::from_rgb(147, 51, 234)
            } else {
                Color32::from_rgb(198, 120, 221)
            }
        }
        _ => {
            if is_light {
                Color32::from_rgb(13, 148, 136)
            } else {
                Color32::from_rgb(86, 182, 194)
            }
        }
    }
}
