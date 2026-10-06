use super::particles::{ember_color, Particle};
use eframe::egui::{pos2, vec2, Color32, Painter, Pos2, Rect};

pub fn emit_fire(particles: &mut Vec<Particle>, pos: Pos2, w: f32, n: usize) {
    for _ in 0..n {
        particles.push(Particle {
            pos: pos2(pos.x + w * 0.5 + (fastrand::f32() - 0.5) * 1.5, pos.y - 0.5),
            vel: vec2(
                (fastrand::f32() - 0.5) * 6.0,
                -(16.0 + fastrand::f32() * 22.0),
            ),
            age: 0.0,
            life: 0.18 + fastrand::f32() * 0.12,
            size: 0.8 + fastrand::f32() * 0.5,
        });
    }
}

pub fn paint_fire(p: &Painter, pos: Pos2, w: f32, lh: f32, now: f64, particles: &[Particle]) {
    // Pure small fire: clean warm flame beam
    let flick = 0.90 + 0.10 * ((now * 18.0).sin() as f32);
    p.rect_filled(
        Rect::from_min_size(pos, vec2(w, lh)),
        1.0,
        Color32::from_rgb((255.0 * flick) as u8, (155.0 * flick) as u8, 30),
    );
    // Delicate micro-embers strictly above the caret tip
    for s in particles {
        let t = s.age / s.life;
        p.circle_filled(s.pos, s.size * (1.0 - 0.5 * t), ember_color(t));
    }
}
