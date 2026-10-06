use super::particles::{ember_color, Particle};
use eframe::egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Stroke};

pub fn paint_candle(
    p: &Painter,
    pos: Pos2,
    w: f32,
    lh: f32,
    now: f64,
    particles: &[Particle],
    is_light: bool,
) {
    // Beeswax amber on light themes, ivory wax on dark themes
    let wax_color = if is_light {
        Color32::from_rgb(195, 120, 20)
    } else {
        Color32::from_rgb(248, 242, 228)
    };
    p.rect_filled(Rect::from_min_size(pos, vec2(w, lh)), 1.0, wax_color);

    let wick_x = pos.x + w * 0.5;
    let wick_top = pos.y - 2.5;

    // Candle wick
    p.line_segment(
        [pos2(wick_x, pos.y), pos2(wick_x, wick_top)],
        Stroke::new(1.0_f32, Color32::from_rgb(50, 40, 30)),
    );

    // Dynamic flame flicker (living animation resting and typing)
    let flick_x = ((now * 16.0).sin() * 0.6) as f32;
    let flick_y = ((now * 22.0).cos() * 0.8) as f32;
    let flame_h = 7.5 + flick_y;

    // Outer warm flame
    let outer_rect = Rect::from_min_max(
        pos2(wick_x - 2.2, wick_top - flame_h * 0.8),
        pos2(wick_x + 2.2, wick_top),
    );
    p.rect_filled(outer_rect, 2.0, Color32::from_rgb(255, 140, 20));

    // Inner bright core
    let inner_rect = Rect::from_min_max(
        pos2(wick_x - 1.2, wick_top - flame_h * 0.55),
        pos2(wick_x + 1.2, wick_top),
    );
    p.rect_filled(inner_rect, 1.0, Color32::from_rgb(255, 245, 160));

    // Flame tip spark
    p.circle_filled(
        pos2(wick_x + flick_x, wick_top - flame_h),
        1.0,
        Color32::from_rgb(255, 255, 230),
    );

    // Warm ambient halo (soft glow)
    p.circle_filled(
        pos2(wick_x, wick_top - flame_h * 0.5),
        6.5 + flick_y * 0.5,
        Color32::from_rgba_unmultiplied(255, 170, 40, 22),
    );

    // Gentle micro-embers
    for s in particles {
        let t = s.age / s.life;
        p.circle_filled(s.pos, s.size * (1.0 - 0.5 * t), ember_color(t));
    }
}
