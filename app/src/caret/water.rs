use super::particles::{Particle, Ripple};
use eframe::egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Stroke};

pub fn emit_drops(particles: &mut Vec<Particle>, pos: Pos2, w: f32, lh: f32, n: usize) {
    for _ in 0..n {
        particles.push(Particle {
            pos: pos2(pos.x + w * 0.5, pos.y + lh - 1.5),
            vel: vec2(
                (fastrand::f32() - 0.5) * 12.0,
                -(10.0 + fastrand::f32() * 14.0),
            ),
            age: 0.0,
            life: 0.22 + fastrand::f32() * 0.14,
            size: 0.9 + fastrand::f32() * 0.4,
        });
    }
}

pub fn paint_water(
    p: &Painter,
    pos: Pos2,
    w: f32,
    lh: f32,
    ripples: &[Ripple],
    particles: &[Particle],
) {
    // Pure water: crystal-clear azure beam
    p.rect_filled(
        Rect::from_min_size(pos, vec2(w, lh)),
        1.0,
        Color32::from_rgb(65, 175, 255),
    );

    // Subtle compact baseline ripple (max 7px, never obscures text)
    for r in ripples {
        let t = r.age / 0.35;
        let a = ((1.0 - t) * 160.0) as u8;
        let radius = 2.0 + t * 6.5;
        p.circle_stroke(
            r.center,
            radius,
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(80, 200, 255, a)),
        );
    }

    // Tiny micro-droplets bouncing near the baseline
    for d in particles {
        let t = d.age / d.life;
        let a = ((1.0 - t) * 200.0) as u8;
        p.circle_filled(
            d.pos,
            d.size * (1.0 - 0.3 * t),
            Color32::from_rgba_unmultiplied(120, 215, 255, a),
        );
    }
}
