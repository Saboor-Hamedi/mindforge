use super::particles::{Bolt, Particle};
use eframe::egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Stroke};
use std::collections::VecDeque;

pub fn make_bolt(pos: Pos2, cw: f32, lh: f32) -> Bolt {
    let mut pts = Vec::with_capacity(7);
    let mut curr = pos2(pos.x + (fastrand::f32() - 0.5) * cw * 0.8, pos.y);
    pts.push(curr);
    for i in 1..=5 {
        let seg_y = pos.y + (lh / 5.0) * i as f32;
        let seg_x = pos.x + (fastrand::f32() - 0.5) * 8.0;
        curr = pos2(seg_x, seg_y);
        pts.push(curr);
    }
    Bolt { pts, age: 0.0 }
}

pub fn emit_matrix(particles: &mut Vec<Particle>, pos: Pos2, w: f32, n: usize) {
    for _ in 0..n {
        particles.push(Particle {
            pos: pos2(pos.x + (fastrand::f32() - 0.5) * w, pos.y),
            vel: vec2(0.0, 30.0 + fastrand::f32() * 40.0),
            age: 0.0,
            life: 0.3 + fastrand::f32() * 0.25,
            size: 2.0 + fastrand::f32() * 2.0,
        });
    }
}

pub fn paint_electric(p: &Painter, pos: Pos2, w: f32, lh: f32, bolts: &[Bolt]) {
    p.rect_filled(
        Rect::from_min_size(pos, vec2(w, lh)),
        0.0,
        Color32::from_rgb(170, 210, 255),
    );
    for b in bolts {
        let a = ((1.0 - b.age / 0.12) * 255.0) as u8;
        p.add(eframe::egui::Shape::line(
            b.pts.clone(),
            Stroke::new(1.5_f32, Color32::from_rgba_unmultiplied(210, 235, 255, a)),
        ));
    }
}

pub fn paint_comet(
    p: &Painter,
    pos: Pos2,
    w: f32,
    lh: f32,
    trail: &VecDeque<Pos2>,
    accent: Color32,
) {
    let n = trail.len().max(1) as f32;
    for (i, tp) in trail.iter().enumerate() {
        let t = i as f32 / n;
        let a = ((1.0 - t) * 150.0) as u8;
        p.rect_filled(
            Rect::from_min_size(*tp, vec2(w, lh)),
            0.0,
            Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), a),
        );
    }
    p.rect_filled(Rect::from_min_size(pos, vec2(w, lh)), 0.0, accent);
}

pub fn paint_matrix(p: &Painter, pos: Pos2, w: f32, lh: f32, particles: &[Particle]) {
    p.rect_filled(
        Rect::from_min_size(pos, vec2(w, lh)),
        0.0,
        Color32::from_rgb(40, 255, 90),
    );
    for p_item in particles {
        let t = p_item.age / p_item.life;
        let a = ((1.0 - t) * 220.0) as u8;
        p.rect_filled(
            Rect::from_min_size(p_item.pos, vec2(w * 0.7, p_item.size * 1.8)),
            0.0,
            Color32::from_rgba_unmultiplied(60, 255, 120, a),
        );
    }
}

pub fn paint_ice(p: &Painter, pos: Pos2, w: f32, lh: f32, now: f64) {
    let frost_color = Color32::from_rgb(135, 230, 255);
    p.rect_filled(Rect::from_min_size(pos, vec2(w, lh)), 1.0, frost_color);
    let glint_alpha = (130.0 + 75.0 * ((now * 5.0).sin() as f32)) as u8;
    p.circle_filled(
        pos2(pos.x + w * 0.5, pos.y + 1.5),
        w * 0.5,
        Color32::from_rgba_unmultiplied(255, 255, 255, glint_alpha),
    );
    p.circle_filled(
        pos2(pos.x + w * 0.5, pos.y + lh - 1.5),
        w * 0.5,
        Color32::from_rgba_unmultiplied(255, 255, 255, glint_alpha),
    );
}

pub fn paint_glitch(p: &Painter, pos: Pos2, w: f32, lh: f32, now: f64, last_type: f64) {
    let is_glitching = (now - last_type) < 0.1;
    if is_glitching {
        let shift = ((now * 60.0).sin() * 2.0) as f32;
        let r_rect = Rect::from_min_size(pos2(pos.x - shift, pos.y), vec2(w, lh));
        let b_rect = Rect::from_min_size(pos2(pos.x + shift, pos.y), vec2(w, lh));
        p.rect_filled(
            r_rect,
            0.0,
            Color32::from_rgba_unmultiplied(255, 30, 60, 160),
        );
        p.rect_filled(
            b_rect,
            0.0,
            Color32::from_rgba_unmultiplied(30, 120, 255, 160),
        );
    }
    p.rect_filled(Rect::from_min_size(pos, vec2(w, lh)), 0.0, Color32::WHITE);
}

pub fn paint_heartbeat(p: &Painter, pos: Pos2, w: f32, lh: f32, now: f64) {
    let pulse = ((now * 5.0).sin() as f32).max(0.0);
    let extra_w = pulse * 1.5;
    let extra_h = pulse * 1.5;
    let heart_rect = Rect::from_min_size(
        pos2(pos.x - extra_w * 0.5, pos.y - extra_h * 0.5),
        vec2(w + extra_w, lh + extra_h),
    );
    p.rect_filled(heart_rect, 1.0, Color32::from_rgb(255, 60, 100));
}
