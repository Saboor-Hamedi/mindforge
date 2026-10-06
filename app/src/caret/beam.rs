use eframe::egui::{vec2, Color32, Painter, Pos2, Rect};

pub fn paint_block(p: &Painter, pos: Pos2, cw: f32, lh: f32, color: Color32) {
    p.rect_filled(Rect::from_min_size(pos, vec2(cw, lh)), 0.0, color);
}

pub fn paint_beam(p: &Painter, pos: Pos2, w: f32, lh: f32, color: Color32) {
    // Subtle soft glow behind the thin vertical beam
    let glow_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 45);
    p.rect_filled(
        Rect::from_min_size(eframe::egui::pos2(pos.x - 1.5, pos.y), vec2(w + 3.0, lh)),
        2.0,
        glow_color,
    );
    p.rect_filled(Rect::from_min_size(pos, vec2(w, lh)), 1.0, color);
}

pub fn paint_underline(p: &Painter, pos: Pos2, cw: f32, w: f32, lh: f32, color: Color32) {
    let y = pos.y + lh - w;
    p.rect_filled(
        Rect::from_min_size(eframe::egui::pos2(pos.x, y), vec2(cw, w)),
        0.0,
        color,
    );
}
