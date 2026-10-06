//! Full-screen Webscan report view and error view.
//!
//! Displays the results of a web security scan in a terminal-inspired format:
//! - Dark background, monospace font, accent color for headers
//! - Findings grouped by category (Headers, Cookies, TLS, Disclosure, Injection, Outdated)
//! - Severity coloring: Low → dim gray, Medium → amber, High → red
//! - Mouse wheel scrolling with automatic scroll clamping
//! - 'q' or 'Esc' returns to previous mode
//! - No popups, no default egui widgets — pure painter rendering
//!
//! Also provides `export_scan_to_markdown` for saving reports as Markdown files.

use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, Align2, Color32, FontId, Rect, Stroke};
use webscan::{Category, Finding, ScanResult, Severity};

/// Renders the full-screen webscan report or error view.
///
/// # Arguments
/// * `ui` — egui UI context (used for scroll input detection)
/// * `painter` — egui painter for all drawing operations
/// * `rect` — Screen area to render within
/// * `result` — The scan result to display (None shows "no report" message)
/// * `error` — Optional (url, error_message) tuple for error display
/// * `scroll_y` — Mutable scroll offset (auto-clamped to content bounds)
/// * `theme` — Active color theme
/// * `font_size` — Base font size for the report
pub fn render_scan_view(
    ui: &egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    result: Option<&ScanResult>,
    error: Option<(&str, &str)>,
    scroll_y: &mut f32,
    theme: &Theme,
    font_size: f32,
) {
    let p = painter.with_clip_rect(rect);
    p.rect_filled(rect, 0.0, theme.bg);

    let pad_x = 24.0;
    let pad_y = 20.0;
    let start_x = rect.min.x + pad_x;
    let mut current_y = rect.min.y + pad_y - *scroll_y;
    let max_w = (rect.width() - pad_x * 2.0).max(100.0);

    // Mouse scroll handling
    if ui.rect_contains_pointer(rect) {
        let delta = ui.input(|i| {
            if i.smooth_scroll_delta.y.abs() > 0.001 {
                i.smooth_scroll_delta.y
            } else {
                i.raw_scroll_delta.y * 0.5
            }
        });
        if delta != 0.0 {
            *scroll_y = (*scroll_y - delta).max(0.0);
        }
    }

    // ── Error State ──────────────────────────────────────────────────────────
    if let Some((url, err_msg)) = error {
        p.text(
            pos2(start_x, current_y),
            Align2::LEFT_TOP,
            "🌐 WEBSCAN FAILED",
            FontId::monospace(font_size * 1.35),
            Color32::from_rgb(235, 87, 87),
        );
        current_y += (font_size * 2.0).round();

        p.text(
            pos2(start_x, current_y),
            Align2::LEFT_TOP,
            format!("Target URL: {}", url),
            FontId::monospace(font_size * 1.05),
            theme.highlight,
        );
        current_y += (font_size * 1.8).round();

        // Divider
        p.line_segment(
            [pos2(start_x, current_y), pos2(start_x + max_w, current_y)],
            Stroke::new(
                1.0_f32,
                Color32::from_rgba_unmultiplied(
                    theme.muted.r(),
                    theme.muted.g(),
                    theme.muted.b(),
                    40,
                ),
            ),
        );
        current_y += 16.0;

        let err_galley = painter.layout(
            format!("Error: {}", err_msg),
            FontId::monospace(font_size),
            Color32::from_rgb(235, 87, 87),
            max_w,
        );
        let eh = err_galley.size().y;
        p.galley(
            pos2(start_x, current_y),
            err_galley,
            Color32::from_rgb(235, 87, 87),
        );
        current_y += eh + 24.0;

        p.text(
            pos2(start_x, current_y),
            Align2::LEFT_TOP,
            "Press 'q' or 'Esc' to return to editor",
            FontId::monospace(font_size * 0.90),
            theme.muted,
        );
        return;
    }

    // ── Success Report State ─────────────────────────────────────────────────
    let res = match result {
        Some(r) => r,
        None => {
            p.text(
                pos2(start_x, current_y),
                Align2::LEFT_TOP,
                "No scan report loaded.",
                FontId::monospace(font_size),
                theme.muted,
            );
            return;
        }
    };

    // Header Title
    p.text(
        pos2(start_x, current_y),
        Align2::LEFT_TOP,
        "🌐 WEBSCAN REPORT",
        FontId::monospace(font_size * 1.35),
        theme.accent,
    );
    current_y += (font_size * 1.8).round();

    // Top block: Target URL
    p.text(
        pos2(start_x, current_y),
        Align2::LEFT_TOP,
        format!("URL: {}", res.url),
        FontId::monospace(font_size * 1.10),
        theme.highlight,
    );
    current_y += (font_size * 1.5).round();

    // Summary pills / metadata line
    let _status_color = if res.status_code < 400 {
        Color32::from_rgb(80, 250, 123)
    } else {
        Color32::from_rgb(255, 85, 85)
    };
    let size_kb = (res.page_size_bytes as f32) / 1024.0;
    let srv_label = res.server_header.as_deref().unwrap_or("None / Hidden");

    let meta_str = format!(
        "HTTP Status: {}   •   Response Time: {}ms   •   Page Size: {:.1} KB   •   Server: {}",
        res.status_code, res.response_time_ms, size_kb, srv_label
    );
    p.text(
        pos2(start_x, current_y),
        Align2::LEFT_TOP,
        meta_str,
        FontId::monospace(font_size * 0.92),
        theme.text,
    );
    current_y += (font_size * 1.4).round();

    // TLS Info line
    let tls_str = if let Some(tls) = &res.tls {
        let proto = tls.protocol.as_deref().unwrap_or("HTTPS (TLS)");
        let cipher = tls.cipher.as_deref().unwrap_or("Default Suite");
        format!("Transport Security: {} [{}]", proto, cipher)
    } else {
        "Transport Security: Unencrypted HTTP (Insecure)".to_string()
    };
    let tls_color = if res.tls.is_some() {
        Color32::from_rgb(97, 175, 239)
    } else {
        Color32::from_rgb(255, 175, 45)
    };
    p.text(
        pos2(start_x, current_y),
        Align2::LEFT_TOP,
        tls_str,
        FontId::monospace(font_size * 0.90),
        tls_color,
    );
    current_y += (font_size * 1.4).round();

    // Note line if present
    if let Some(note) = &res.note {
        p.text(
            pos2(start_x, current_y),
            Align2::LEFT_TOP,
            format!("Note: {}", note),
            FontId::monospace(font_size * 0.90),
            theme.muted,
        );
        current_y += (font_size * 1.4).round();
    }

    current_y += 6.0;

    // Top divider
    p.line_segment(
        [pos2(start_x, current_y), pos2(start_x + max_w, current_y)],
        Stroke::new(
            1.0_f32,
            Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 40),
        ),
    );
    current_y += 18.0;

    // Findings section
    let categories = [
        Category::Headers,
        Category::Cookies,
        Category::Tls,
        Category::Disclosure,
        Category::Injection,
        Category::Outdated,
    ];

    let total_findings = res.findings.len();
    if total_findings == 0 {
        p.text(
            pos2(start_x, current_y),
            Align2::LEFT_TOP,
            "✓ No vulnerabilities or security misconfigurations detected.",
            FontId::monospace(font_size * 1.05),
            Color32::from_rgb(80, 250, 123),
        );
        current_y += (font_size * 2.0).round();
    } else {
        p.text(
            pos2(start_x, current_y),
            Align2::LEFT_TOP,
            format!("FINDINGS ({})", total_findings),
            FontId::monospace(font_size * 1.15),
            theme.highlight,
        );
        current_y += (font_size * 1.8).round();

        for cat in &categories {
            let cat_findings: Vec<&Finding> =
                res.findings.iter().filter(|f| f.category == *cat).collect();
            if cat_findings.is_empty() {
                continue;
            }

            // Category section header
            p.text(
                pos2(start_x, current_y),
                Align2::LEFT_TOP,
                format!("── {} ────────────────────────────", cat.title()),
                FontId::monospace(font_size * 1.0),
                theme.accent,
            );
            current_y += (font_size * 1.5).round();

            for f in cat_findings {
                let (badge_color, badge_text) = match f.severity {
                    Severity::High => (Color32::from_rgb(235, 87, 87), "[HIGH]"),
                    Severity::Medium => (Color32::from_rgb(255, 175, 45), "[MED] "),
                    Severity::Low => (Color32::from_gray(140), "[LOW] "),
                };

                let line_str = format!("{} {}  —  {}", badge_text, f.title, f.description);
                let galley = painter.layout(
                    line_str,
                    FontId::monospace(font_size * 0.92),
                    badge_color,
                    max_w - 16.0,
                );
                let gh = galley.size().y;
                if current_y + gh >= rect.min.y && current_y <= rect.max.y {
                    p.galley(pos2(start_x + 8.0, current_y), galley, badge_color);
                }
                current_y += gh + 6.0;
            }

            current_y += 10.0;
        }
    }

    current_y += 16.0;
    p.line_segment(
        [pos2(start_x, current_y), pos2(start_x + max_w, current_y)],
        Stroke::new(
            1.0_f32,
            Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 25),
        ),
    );
    current_y += 14.0;

    // Navigation hints
    p.text(
        pos2(start_x, current_y),
        Align2::LEFT_TOP,
        "Commands: [q] or [Esc] to return to editor   •   [:export] save as markdown   •   [:scans] history",
        FontId::monospace(font_size * 0.88),
        theme.muted,
    );
    current_y += 24.0;

    // Clamp scroll
    let total_h = (current_y + *scroll_y - (rect.min.y + pad_y)).max(0.0);
    let max_scroll = (total_h - rect.height() + pad_y * 2.0).max(0.0);
    *scroll_y = scroll_y.clamp(0.0, max_scroll);
}

/// Generates a structured Markdown export of a scan report.
///
/// Produces a `.md`-ready string with target metadata, TLS info,
/// and a bulleted findings list grouped by severity.
pub fn export_scan_to_markdown(res: &ScanResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Webscan Report: {}\n\n", res.url));
    out.push_str(&format!("- **Target URL:** {}\n", res.url));
    out.push_str(&format!("- **HTTP Status:** {}\n", res.status_code));
    out.push_str(&format!(
        "- **Response Time:** {} ms\n",
        res.response_time_ms
    ));
    out.push_str(&format!("- **Page Size:** {} bytes\n", res.page_size_bytes));
    if let Some(srv) = &res.server_header {
        out.push_str(&format!("- **Server:** {}\n", srv));
    }
    if let Some(tls) = &res.tls {
        out.push_str(&format!(
            "- **TLS:** {}\n",
            tls.protocol.as_deref().unwrap_or("HTTPS")
        ));
    } else {
        out.push_str("- **TLS:** Unencrypted HTTP\n");
    }
    if let Some(note) = &res.note {
        out.push_str(&format!("- **Note:** {}\n", note));
    }

    out.push_str("\n## Findings\n\n");
    if res.findings.is_empty() {
        out.push_str("No security issues detected.\n");
    } else {
        for f in &res.findings {
            out.push_str(&format!(
                "- **[{:?}] [{}]** {}: {}\n",
                f.category,
                f.severity.as_str(),
                f.title,
                f.description
            ));
        }
    }

    out
}
