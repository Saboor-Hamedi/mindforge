//! Window layout geometry, 5px uniform gap calculations, and panel bounds.

use eframe::egui::{pos2, vec2, Rect};

pub const GAP: f32 = 5.0;
pub const TITLEBAR_H: f32 = 32.0;
pub const CMD_BAR_H: f32 = 32.0;
pub const SPLITTER_BAR_W: f32 = 1.0;
pub const MIN_SIDEBAR_W: f32 = 170.0;
pub const MAX_SIDEBAR_W: f32 = 480.0;
pub const DEFAULT_SIDEBAR_W: f32 = 230.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppLayout {
    pub titlebar_rect: Rect,
    pub cmd_bar_rect: Rect,
    pub sidebar_rect: Option<Rect>,
    pub splitter_hit_rect: Option<Rect>,
    pub splitter_center_x: Option<f32>,
    pub right_sidebar_rect: Option<Rect>,
    pub editor_panel_rect: Rect,
}

/// Computes the exact window geometry ensuring a uniform 5px outer gap everywhere,
/// identical top and bottom alignment between the sidebar and editor panel, and
/// a centered resizable splitter knob. Supports modular hiding of titlebar, statusbar, and right sidebar.
pub fn compute_modular_layout(
    bounds: Rect,
    sidebar_open: bool,
    sidebar_w: f32,
    show_titlebar: bool,
    show_statusbar: bool,
) -> AppLayout {
    compute_modular_layout_ex(
        bounds,
        sidebar_open,
        sidebar_w,
        false,
        0.0,
        show_titlebar,
        show_statusbar,
    )
}

/// Computes layout including optional right sidebar inspector.
pub fn compute_modular_layout_ex(
    bounds: Rect,
    sidebar_open: bool,
    sidebar_w: f32,
    right_sidebar_open: bool,
    right_sidebar_w: f32,
    show_titlebar: bool,
    show_statusbar: bool,
) -> AppLayout {
    let titlebar_rect = if show_titlebar {
        Rect::from_min_max(
            pos2(bounds.min.x + GAP, bounds.min.y + GAP),
            pos2(bounds.max.x - GAP, bounds.min.y + GAP + TITLEBAR_H),
        )
    } else {
        Rect::NOTHING
    };

    let cmd_bar_rect = if show_statusbar {
        Rect::from_min_max(
            pos2(bounds.min.x + GAP, bounds.max.y - GAP - CMD_BAR_H),
            pos2(bounds.max.x - GAP, bounds.max.y - GAP),
        )
    } else {
        Rect::NOTHING
    };

    let panel_top = if show_titlebar {
        titlebar_rect.max.y + GAP
    } else {
        bounds.min.y + GAP
    };

    let panel_bottom = if show_statusbar {
        cmd_bar_rect.min.y - GAP
    } else {
        bounds.max.y - GAP
    };

    let right_sidebar_rect = if right_sidebar_open {
        let rw = right_sidebar_w.clamp(MIN_SIDEBAR_W, MAX_SIDEBAR_W);
        Some(Rect::from_min_max(
            pos2(bounds.max.x - GAP - rw, panel_top),
            pos2(bounds.max.x - GAP, panel_bottom),
        ))
    } else {
        None
    };

    let ed_right = if let Some(rsb) = right_sidebar_rect {
        rsb.min.x - GAP
    } else {
        bounds.max.x - GAP
    };

    if sidebar_open {
        let sw = sidebar_w.clamp(MIN_SIDEBAR_W, MAX_SIDEBAR_W);
        let sb_rect = Rect::from_min_max(
            pos2(bounds.min.x + GAP, panel_top),
            pos2(bounds.min.x + GAP + sw, panel_bottom),
        );
        let splitter_center_x = sb_rect.max.x + GAP + SPLITTER_BAR_W * 0.5;
        let split_hit_rect = Rect::from_center_size(
            pos2(splitter_center_x, (panel_top + panel_bottom) * 0.5),
            vec2(14.0, (panel_bottom - panel_top).max(0.0)),
        );
        let ed_left = sb_rect.max.x + GAP + SPLITTER_BAR_W + GAP;
        let ed_panel = Rect::from_min_max(pos2(ed_left, panel_top), pos2(ed_right, panel_bottom));
        AppLayout {
            titlebar_rect,
            cmd_bar_rect,
            sidebar_rect: Some(sb_rect),
            splitter_hit_rect: Some(split_hit_rect),
            splitter_center_x: Some(splitter_center_x),
            right_sidebar_rect,
            editor_panel_rect: ed_panel,
        }
    } else {
        let ed_panel = Rect::from_min_max(
            pos2(bounds.min.x + GAP, panel_top),
            pos2(ed_right, panel_bottom),
        );
        AppLayout {
            titlebar_rect,
            cmd_bar_rect,
            sidebar_rect: None,
            splitter_hit_rect: None,
            splitter_center_x: None,
            right_sidebar_rect,
            editor_panel_rect: ed_panel,
        }
    }
}

pub fn compute_app_layout(bounds: Rect, sidebar_open: bool, sidebar_w: f32) -> AppLayout {
    compute_modular_layout(bounds, sidebar_open, sidebar_w, true, true)
}

pub fn compute_zen_layout(bounds: Rect, sidebar_open: bool, sidebar_w: f32) -> AppLayout {
    compute_modular_layout(bounds, sidebar_open, sidebar_w, false, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zen_layout_bounds() {
        let bounds = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0));
        let layout_closed = compute_zen_layout(bounds, false, 230.0);

        assert_eq!(layout_closed.editor_panel_rect.min.x - bounds.min.x, GAP);
        assert_eq!(layout_closed.editor_panel_rect.min.y - bounds.min.y, GAP);
        assert_eq!(bounds.max.x - layout_closed.editor_panel_rect.max.x, GAP);
        assert_eq!(
            layout_closed.cmd_bar_rect.min.y - layout_closed.editor_panel_rect.max.y,
            GAP
        );
        assert_eq!(bounds.max.y - layout_closed.cmd_bar_rect.max.y, GAP);
        assert_eq!(layout_closed.sidebar_rect, None);
        assert_eq!(layout_closed.titlebar_rect, Rect::NOTHING);

        // Zen mode with sidebar opened
        let layout_open = compute_zen_layout(bounds, true, 230.0);
        assert!(layout_open.sidebar_rect.is_some());
        assert_eq!(layout_open.sidebar_rect.unwrap().min.y - bounds.min.y, GAP);
        assert_eq!(layout_open.sidebar_rect.unwrap().width(), 230.0);
        assert_eq!(layout_open.titlebar_rect, Rect::NOTHING);
    }

    #[test]
    fn test_uniform_5px_margins_sidebar_open() {
        let bounds = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0));
        let layout = compute_app_layout(bounds, true, 230.0);

        // Window edge insets must all be exactly 5.0
        assert_eq!(layout.titlebar_rect.min.x - bounds.min.x, GAP);
        assert_eq!(layout.titlebar_rect.min.y - bounds.min.y, GAP);
        assert_eq!(bounds.max.x - layout.titlebar_rect.max.x, GAP);

        assert_eq!(layout.cmd_bar_rect.min.x - bounds.min.x, GAP);
        assert_eq!(bounds.max.x - layout.cmd_bar_rect.max.x, GAP);
        assert_eq!(bounds.max.y - layout.cmd_bar_rect.max.y, GAP);

        // Sidebar insets
        let sb = layout.sidebar_rect.unwrap();
        assert_eq!(sb.min.x - bounds.min.x, GAP);
        assert_eq!(sb.min.y - layout.titlebar_rect.max.y, GAP);
        assert_eq!(layout.cmd_bar_rect.min.y - sb.max.y, GAP);

        // Editor panel insets
        let ed = layout.editor_panel_rect;
        assert_eq!(bounds.max.x - ed.max.x, GAP);
        assert_eq!(ed.min.y - layout.titlebar_rect.max.y, GAP);
        assert_eq!(layout.cmd_bar_rect.min.y - ed.max.y, GAP);

        // Gaps around splitter: sb -> splitter (5px), splitter -> ed (5px)
        let splitter_center = layout.splitter_center_x.unwrap();
        assert_eq!(splitter_center - SPLITTER_BAR_W * 0.5 - sb.max.x, GAP);
        assert_eq!(ed.min.x - (splitter_center + SPLITTER_BAR_W * 0.5), GAP);
    }

    #[test]
    fn test_sidebar_and_editor_align_top_and_bottom_identically() {
        for height in [500.0, 700.0, 900.0, 1200.0] {
            for width in [800.0, 1120.0, 1600.0] {
                for sw in [170.0, 230.0, 350.0, 480.0] {
                    let bounds = Rect::from_min_size(pos2(10.0, 20.0), vec2(width, height));
                    let layout = compute_app_layout(bounds, true, sw);
                    let sb = layout.sidebar_rect.unwrap();
                    let ed = layout.editor_panel_rect;

                    // Critical assertion per spec: top and bottom must align with 0 difference
                    assert_eq!(
                        sb.min.y, ed.min.y,
                        "Sidebar top must match editor panel top"
                    );
                    assert_eq!(
                        sb.max.y, ed.max.y,
                        "Sidebar bottom must match editor panel bottom"
                    );
                    assert_eq!(
                        sb.height(),
                        ed.height(),
                        "Sidebar height must match editor panel height"
                    );
                }
            }
        }
    }

    #[test]
    fn test_sidebar_clamp_limits() {
        let bounds = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0));
        let layout_min = compute_app_layout(bounds, true, 50.0);
        assert_eq!(layout_min.sidebar_rect.unwrap().width(), MIN_SIDEBAR_W);

        let layout_max = compute_app_layout(bounds, true, 900.0);
        assert_eq!(layout_max.sidebar_rect.unwrap().width(), MAX_SIDEBAR_W);
    }
}
