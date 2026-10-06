use super::particles::Particle;
use eframe::egui::{pos2, vec2, Color32, Painter, Pos2, Rect};

pub fn emit_snow(particles: &mut Vec<Particle>, pos: Pos2, w: f32, _lh: f32, n: usize) {
    for _ in 0..n {
        particles.push(Particle {
            pos: pos2(
                pos.x + w * 0.5 + (fastrand::f32() - 0.5) * 1.5,
                pos.y - 1.0 - fastrand::f32() * 3.0,
            ),
            vel: vec2((fastrand::f32() - 0.5) * 4.0, 14.0 + fastrand::f32() * 18.0),
            age: 0.0,
            life: 0.45 + fastrand::f32() * 0.35,
            size: 0.8 + fastrand::f32() * 0.6,
        });
    }
}

pub fn paint_snow(p: &Painter, pos: Pos2, w: f32, lh: f32, particles: &[Particle], is_light: bool) {
    // On light themes use deep crisp arctic blue; on dark themes use pure crystalline white
    let beam_color = if is_light {
        Color32::from_rgb(35, 115, 185)
    } else {
        Color32::from_rgb(225, 245, 255)
    };
    p.rect_filled(Rect::from_min_size(pos, vec2(w, lh)), 1.0, beam_color);

    // Delicate falling snowflakes drifting gently down along the caret
    for s in particles {
        let t = s.age / s.life;
        let a = ((1.0 - t) * 220.0) as u8;
        let flake_color = if is_light {
            Color32::from_rgba_unmultiplied(45, 130, 210, a)
        } else {
            Color32::from_rgba_unmultiplied(245, 250, 255, a)
        };
        p.circle_filled(s.pos, s.size * (1.0 - 0.2 * t), flake_color);
    }
}
