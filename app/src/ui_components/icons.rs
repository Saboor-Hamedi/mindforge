//! Modern vector icons rendered directly via egui Painter.
//! Replaces native OS emojis with crisp, theme-aware dual-tone geometric icons.

use eframe::egui::{pos2, vec2, Color32, Painter, Rect, Stroke};

/// Renders modern vector book / codex icon.
pub fn render_book_icon(painter: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let w = rect.width();
    let h = rect.height();
    let stroke = Stroke::new(1.3_f32, color);

    // Spine
    painter.line_segment(
        [pos2(c.x, c.y - h * 0.38), pos2(c.x, c.y + h * 0.42)],
        stroke,
    );

    // Left page outline
    let left_pts = vec![
        pos2(c.x, c.y - h * 0.38),
        pos2(c.x - w * 0.42, c.y - h * 0.28),
        pos2(c.x - w * 0.42, c.y + h * 0.32),
        pos2(c.x, c.y + h * 0.42),
    ];
    for w in left_pts.windows(2) {
        painter.line_segment([w[0], w[1]], stroke);
    }

    // Right page outline
    let right_pts = vec![
        pos2(c.x, c.y - h * 0.38),
        pos2(c.x + w * 0.42, c.y - h * 0.28),
        pos2(c.x + w * 0.42, c.y + h * 0.32),
        pos2(c.x, c.y + h * 0.42),
    ];
    for w in right_pts.windows(2) {
        painter.line_segment([w[0], w[1]], stroke);
    }

    // Inner subtle page line
    let dim_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 110);
    painter.line_segment(
        [
            pos2(c.x - w * 0.28, c.y - h * 0.05),
            pos2(c.x - w * 0.12, c.y - h * 0.02),
        ],
        Stroke::new(1.0_f32, dim_color),
    );
    painter.line_segment(
        [
            pos2(c.x + w * 0.12, c.y - h * 0.02),
            pos2(c.x + w * 0.28, c.y - h * 0.05),
        ],
        Stroke::new(1.0_f32, dim_color),
    );
}

/// Renders modern art palette icon with vibrant colorful pigment wells.
pub fn render_palette_icon(painter: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.44;
    let stroke = Stroke::new(1.3_f32, color);

    // Oval body
    painter.circle_stroke(c, r, stroke);

    // Thumb hole
    painter.circle_filled(
        pos2(c.x + r * 0.42, c.y + r * 0.38),
        r * 0.22,
        Color32::from_black_alpha(40),
    );
    painter.circle_stroke(
        pos2(c.x + r * 0.42, c.y + r * 0.38),
        r * 0.22,
        Stroke::new(1.0_f32, color),
    );

    // Vibrant colorful pigment dots
    let dot_r = (r * 0.20).clamp(1.5, 3.2);
    // Red dot
    painter.circle_filled(
        pos2(c.x - r * 0.45, c.y - r * 0.32),
        dot_r,
        Color32::from_rgb(255, 95, 87),
    );
    // Amber dot
    painter.circle_filled(
        pos2(c.x - r * 0.15, c.y - r * 0.55),
        dot_r,
        Color32::from_rgb(255, 189, 46),
    );
    // Green dot
    painter.circle_filled(
        pos2(c.x + r * 0.28, c.y - r * 0.42),
        dot_r,
        Color32::from_rgb(39, 201, 63),
    );
    // Cyan dot
    painter.circle_filled(
        pos2(c.x - r * 0.45, c.y + r * 0.25),
        dot_r,
        Color32::from_rgb(45, 175, 255),
    );
}

/// Renders modern speaker / sound icon.
pub fn render_speaker_icon(painter: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let w = rect.width();
    let h = rect.height();
    let stroke = Stroke::new(1.3_f32, color);

    // Speaker base box
    painter.rect_filled(
        Rect::from_min_max(
            pos2(c.x - w * 0.40, c.y - h * 0.20),
            pos2(c.x - w * 0.18, c.y + h * 0.20),
        ),
        1.0,
        color,
    );

    // Speaker cone trapezoid
    let cone = vec![
        pos2(c.x - w * 0.18, c.y - h * 0.20),
        pos2(c.x + w * 0.08, c.y - h * 0.38),
        pos2(c.x + w * 0.08, c.y + h * 0.38),
        pos2(c.x - w * 0.18, c.y + h * 0.20),
    ];
    painter.add(eframe::egui::Shape::convex_polygon(
        cone,
        color,
        Stroke::NONE,
    ));

    // Sound waves
    let wave1_pts = [
        pos2(c.x + w * 0.20, c.y - h * 0.22),
        pos2(c.x + w * 0.26, c.y),
        pos2(c.x + w * 0.20, c.y + h * 0.22),
    ];
    painter.line_segment([wave1_pts[0], wave1_pts[1]], stroke);
    painter.line_segment([wave1_pts[1], wave1_pts[2]], stroke);

    let wave2_pts = [
        pos2(c.x + w * 0.34, c.y - h * 0.36),
        pos2(c.x + w * 0.44, c.y),
        pos2(c.x + w * 0.34, c.y + h * 0.36),
    ];
    let dim_stroke = Stroke::new(
        1.1_f32,
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 160),
    );
    painter.line_segment([wave2_pts[0], wave2_pts[1]], dim_stroke);
    painter.line_segment([wave2_pts[1], wave2_pts[2]], dim_stroke);
}

/// Renders mute speaker icon.
pub fn render_mute_icon(painter: &Painter, rect: Rect, color: Color32) {
    render_speaker_icon(painter, rect, color);
    let c = rect.center();
    let w = rect.width();
    let h = rect.height();
    // Red diagonal slash
    painter.line_segment(
        [
            pos2(c.x - w * 0.35, c.y + h * 0.35),
            pos2(c.x + w * 0.40, c.y - h * 0.35),
        ],
        Stroke::new(1.6_f32, Color32::from_rgb(255, 95, 87)),
    );
}

/// Renders modern four-point sparkle / caret star icon.
pub fn render_sparkle_icon(painter: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.46;
    let inner_r = r * 0.28;

    let pts = vec![
        pos2(c.x, c.y - r),
        pos2(c.x + inner_r, c.y - inner_r),
        pos2(c.x + r, c.y),
        pos2(c.x + inner_r, c.y + inner_r),
        pos2(c.x, c.y + r),
        pos2(c.x - inner_r, c.y + inner_r),
        pos2(c.x - r, c.y),
        pos2(c.x - inner_r, c.y - inner_r),
    ];
    painter.add(eframe::egui::Shape::convex_polygon(
        pts,
        color,
        Stroke::NONE,
    ));

    // Center radiant highlight dot
    painter.circle_filled(c, inner_r * 0.65, Color32::WHITE);
}

/// Renders modern search magnifying glass.
pub fn render_search_icon(painter: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.30;
    let lens_c = pos2(c.x - r * 0.35, c.y - r * 0.35);

    // Glass circle
    painter.circle_stroke(lens_c, r, Stroke::new(1.4_f32, color));
    painter.circle_filled(
        lens_c,
        r - 0.7,
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 25),
    );

    // Handle
    let h_start = pos2(lens_c.x + r * 0.70, lens_c.y + r * 0.70);
    let h_end = pos2(c.x + r * 1.35, c.y + r * 1.35);
    painter.line_segment([h_start, h_end], Stroke::new(1.9_f32, color));
}

/// Renders modern gear / settings icon.
pub fn render_gear_icon(painter: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.42;
    let stroke = Stroke::new(1.3_f32, color);

    // Outer gear rim
    painter.circle_stroke(c, r * 0.82, stroke);
    // Center hole
    painter.circle_stroke(c, r * 0.35, stroke);

    // 6 Cogs
    for i in 0..6 {
        let angle = i as f32 * std::f32::consts::PI / 3.0;
        let dir = vec2(angle.cos(), angle.sin());
        let p1 = c + dir * (r * 0.75);
        let p2 = c + dir * r;
        painter.line_segment([p1, p2], Stroke::new(1.8_f32, color));
    }
}

/// Renders modern document / note icon with folded corner.
pub fn render_document_icon(painter: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let w = rect.width() * 0.72;
    let h = rect.height() * 0.88;
    let fold = w * 0.32;
    let stroke = Stroke::new(1.3_f32, color);

    let min_x = c.x - w * 0.5;
    let max_x = c.x + w * 0.5;
    let min_y = c.y - h * 0.5;
    let max_y = c.y + h * 0.5;

    // Page body with cut corner
    let pts = vec![
        pos2(min_x, min_y),
        pos2(max_x - fold, min_y),
        pos2(max_x, min_y + fold),
        pos2(max_x, max_y),
        pos2(min_x, max_y),
    ];
    for w_pts in pts.windows(2) {
        painter.line_segment([w_pts[0], w_pts[1]], stroke);
    }
    painter.line_segment([pts[4], pts[0]], stroke);

    // Fold flap
    painter.line_segment(
        [pos2(max_x - fold, min_y), pos2(max_x - fold, min_y + fold)],
        stroke,
    );
    painter.line_segment(
        [pos2(max_x - fold, min_y + fold), pos2(max_x, min_y + fold)],
        stroke,
    );

    // 2 Text lines
    let line_col = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 120);
    painter.line_segment(
        [
            pos2(min_x + w * 0.22, c.y + h * 0.05),
            pos2(max_x - w * 0.22, c.y + h * 0.05),
        ],
        Stroke::new(1.0_f32, line_col),
    );
    painter.line_segment(
        [
            pos2(min_x + w * 0.22, c.y + h * 0.24),
            pos2(max_x - w * 0.35, c.y + h * 0.24),
        ],
        Stroke::new(1.0_f32, line_col),
    );
}

/// Renders modern eye / preview icon.
pub fn render_eye_icon(painter: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let w = rect.width() * 0.88;
    let h = rect.height() * 0.55;
    let stroke = Stroke::new(1.3_f32, color);

    let left = pos2(c.x - w * 0.5, c.y);
    let right = pos2(c.x + w * 0.5, c.y);
    let top = pos2(c.x, c.y - h * 0.5);
    let bottom = pos2(c.x, c.y + h * 0.5);

    // Almond curves
    painter.line_segment([left, top], stroke);
    painter.line_segment([top, right], stroke);
    painter.line_segment([right, bottom], stroke);
    painter.line_segment([bottom, left], stroke);

    // Iris circle
    painter.circle_filled(c, h * 0.32, color);
    // Pupil reflection dot
    painter.circle_filled(
        pos2(c.x - h * 0.09, c.y - h * 0.09),
        h * 0.12,
        Color32::WHITE,
    );
}

/// Renders modern lightning bolt / command icon.
pub fn render_bolt_icon(painter: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let w = rect.width() * 0.70;
    let h = rect.height() * 0.90;

    let pts = vec![
        pos2(c.x + w * 0.12, c.y - h * 0.48),
        pos2(c.x - w * 0.40, c.y + h * 0.02),
        pos2(c.x - w * 0.04, c.y + h * 0.02),
        pos2(c.x - w * 0.12, c.y + h * 0.48),
        pos2(c.x + w * 0.40, c.y - h * 0.02),
        pos2(c.x + w * 0.04, c.y - h * 0.02),
    ];
    painter.add(eframe::egui::Shape::convex_polygon(
        pts,
        color,
        Stroke::NONE,
    ));
}

/// Renders modern typography font "A" icon.
pub fn render_font_icon(painter: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let w = rect.width() * 0.70;
    let h = rect.height() * 0.85;
    let stroke = Stroke::new(1.4_f32, color);

    let apex = pos2(c.x, c.y - h * 0.46);
    let left_foot = pos2(c.x - w * 0.42, c.y + h * 0.46);
    let right_foot = pos2(c.x + w * 0.42, c.y + h * 0.46);

    painter.line_segment([apex, left_foot], stroke);
    painter.line_segment([apex, right_foot], stroke);

    // Crossbar
    let bar_left = pos2(c.x - w * 0.22, c.y + h * 0.14);
    let bar_right = pos2(c.x + w * 0.22, c.y + h * 0.14);
    painter.line_segment([bar_left, bar_right], stroke);
}

/// Renders modern pen / writing mode icon.
pub fn render_pen_icon(painter: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let w = rect.width() * 0.65;
    let h = rect.height() * 0.80;

    // Angled pen barrel
    let p_tip = pos2(c.x - w * 0.35, c.y + h * 0.40);
    let p_top = pos2(c.x + w * 0.35, c.y - h * 0.40);
    painter.line_segment([p_tip, p_top], Stroke::new(2.2_f32, color));

    // Nib point
    painter.circle_filled(p_tip, 1.8, Color32::from_rgb(255, 189, 46));
}

/// Unified icon dispatcher that maps string identifiers or emoji glyphs
/// into modern vector graphic icons rendered directly via painter.
pub fn render_vector_icon(painter: &Painter, icon_id: &str, rect: Rect, color: Color32) {
    match icon_id {
        "📖" | "book" | "docs" => render_book_icon(painter, rect, color),
        "🎨" | "theme" | "palette" => render_palette_icon(painter, rect, color),
        "🔊" | "sound" | "audio" => render_speaker_icon(painter, rect, color),
        "🔇" | "mute" => render_mute_icon(painter, rect, color),
        "✦" | "sparkle" | "caret" => render_sparkle_icon(painter, rect, color),
        "🔍" | "search" => render_search_icon(painter, rect, color),
        "⚙" | "settings" | "gear" => render_gear_icon(painter, rect, color),
        "📄" | "doc" | "note" => render_document_icon(painter, rect, color),
        "👁" | "preview" | "view" => render_eye_icon(painter, rect, color),
        "⚡" | "bolt" | "command" => render_bolt_icon(painter, rect, color),
        "🔤" | "font" | "typography" => render_font_icon(painter, rect, color),
        "✍" | "mode" | "pen" => render_pen_icon(painter, rect, color),
        _ => {
            // Text fallback for unmapped strings
            painter.text(
                rect.center(),
                eframe::egui::Align2::CENTER_CENTER,
                icon_id,
                eframe::egui::FontId::monospace(rect.height() * 0.80),
                color,
            );
        }
    }
}
