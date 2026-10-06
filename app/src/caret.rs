//! Caret state, animated styles, particle physics, and coordinate container.

pub mod beam;
pub mod candle;
pub mod effects;
pub mod fire;
pub mod neon;
pub mod particles;
pub mod rainbow;
pub mod snow;
pub mod water;

// Aliases matching user naming patterns
#[allow(unused_imports)]
pub use beam as beamcaret;
#[allow(unused_imports)]
pub use candle as candlecaret;
#[allow(unused_imports)]
pub use effects as effectscaret;
#[allow(unused_imports)]
pub use fire as firecaret;
#[allow(unused_imports)]
pub use neon as neoncaret;
#[allow(unused_imports)]
pub use particles as particlescaret;
#[allow(unused_imports)]
pub use rainbow as rainbowcaret;
#[allow(unused_imports)]
pub use snow as snowcaret;
#[allow(unused_imports)]
pub use water as watercaret;

pub use particles::{Bolt, Particle, Ripple};

use eframe::egui::{Color32, Painter, Pos2};
use std::collections::VecDeque;

/// Selectable caret appearance styles.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CaretKind {
    Block,
    Beam,
    Underline,
    Candle,
    Fire,
    Water,
    Snow,
    Neon,
    Rainbow,
    // Secondary effects
    Electric,
    Comet,
    Matrix,
    Ice,
    Glitch,
    Heartbeat,
}

impl CaretKind {
    /// Curated active collection shown in settings.
    pub const ALL: &'static [CaretKind] = &[
        CaretKind::Block,
        CaretKind::Beam,
        CaretKind::Underline,
        CaretKind::Candle,
        CaretKind::Fire,
        CaretKind::Water,
        CaretKind::Snow,
        CaretKind::Neon,
        CaretKind::Rainbow,
    ];

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.to_lowercase().as_str() {
            "block" => Self::Block,
            "beam" => Self::Beam,
            "underline" => Self::Underline,
            "candle" => Self::Candle,
            "fire" => Self::Fire,
            "water" => Self::Water,
            "snow" => Self::Snow,
            "neon" => Self::Neon,
            "rainbow" => Self::Rainbow,
            "electric" => Self::Electric,
            "comet" => Self::Comet,
            "matrix" => Self::Matrix,
            "ice" => Self::Ice,
            "glitch" => Self::Glitch,
            "heartbeat" => Self::Heartbeat,
            _ => return None,
        })
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Beam => "beam",
            Self::Underline => "underline",
            Self::Candle => "candle",
            Self::Fire => "fire",
            Self::Water => "water",
            Self::Snow => "snow",
            Self::Neon => "neon",
            Self::Rainbow => "rainbow",
            Self::Electric => "electric",
            Self::Comet => "comet",
            Self::Matrix => "matrix",
            Self::Ice => "ice",
            Self::Glitch => "glitch",
            Self::Heartbeat => "heartbeat",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Block => "Classic solid block",
            Self::Beam => "Vertical cursor bar",
            Self::Underline => "Horizontal underscore",
            Self::Candle => "Cozy candle flame with gentle ambient flicker",
            Self::Fire => "Pure small flame & dancing micro-embers",
            Self::Water => "Liquid azure beam & dripping water ripples",
            Self::Snow => "Pure gentle falling snowflakes",
            Self::Neon => "Subtle focused laser aura",
            Self::Rainbow => "Smooth chromatic color spectrum",
            _ => "Animated caret style",
        }
    }
}

/// Unified caret resolution: In Vim mode, respect the user's custom chosen caret kind
/// consistently across Normal, Insert, Visual, and VisualLine modes.
pub fn resolve_caret_kind(_vim_mode: Option<&str>, custom_kind: CaretKind) -> CaretKind {
    custom_kind
}

/// Dynamic caret coordinator managing position, smooth glide, and particle systems.
pub struct Caret {
    pub pos: Pos2,
    pub width: f32,
    pub kind: CaretKind,
    pub glide: f32,
    pub animations_enabled: bool,
    pub blink_enabled: bool,
    pub last_type: f64,
    pub calm: bool,
    pub particles: Vec<Particle>,
    pub ripples: Vec<Ripple>,
    pub bolts: Vec<Bolt>,
    pub trail: VecDeque<Pos2>,
    pub hue: f32,
    pub gliding: bool,
    pub drip_timer: f32,
}

impl Default for Caret {
    fn default() -> Self {
        Self {
            pos: Pos2::ZERO,
            width: 2.0,
            kind: CaretKind::Beam,
            glide: 1.0,
            animations_enabled: true,
            blink_enabled: true,
            last_type: 0.0,
            calm: false,
            particles: Vec::new(),
            ripples: Vec::new(),
            bolts: Vec::new(),
            trail: VecDeque::new(),
            hue: 0.0,
            gliding: false,
            drip_timer: 0.0,
        }
    }
}

impl Caret {
    pub fn new(kind: CaretKind) -> Self {
        Self {
            kind,
            ..Self::default()
        }
    }

    /// Drop transient animation state when another editor backend owns cursor rendering.
    pub fn clear_transient_effects(&mut self) {
        self.gliding = false;
        self.particles.clear();
        self.ripples.clear();
        self.bolts.clear();
        self.trail.clear();
    }

    /// Whether any dynamic elements or ambient animations require ongoing frame repainting.
    pub fn is_animating(&self, _now: f64) -> bool {
        if !self.animations_enabled {
            return false;
        }
        self.gliding
            || !self.particles.is_empty()
            || !self.ripples.is_empty()
            || !self.bolts.is_empty()
            || !self.trail.is_empty()
            || matches!(
                self.kind,
                CaretKind::Candle
                    | CaretKind::Fire
                    | CaretKind::Water
                    | CaretKind::Snow
                    | CaretKind::Rainbow
                    | CaretKind::Ice
                    | CaretKind::Heartbeat
            )
    }

    /// Advance physics, animations, and particle simulation by `dt` seconds.
    pub fn update(&mut self, dt: f32, target: Pos2, typed: bool, now: f64, cw: f32, lh: f32) {
        if !self.animations_enabled {
            self.pos = target;
            self.gliding = false;
            self.particles.clear();
            self.ripples.clear();
            self.bolts.clear();
            self.trail.clear();
            if typed {
                self.last_type = now;
            }
            return;
        }

        // 1. Adaptive smooth glide
        // The cursor position often arrives a frame or more after the keystroke,
        // so keep snapping for a short window after the last typed character.
        let typing = typed || now - self.last_type < 0.4;
        if self.glide.is_finite() && self.glide > 0.0 {
            let diff = target - self.pos;
            if diff.length() > 0.0 {
                if typing {
                    // While typing, snap caret to target instantly so it
                    // always sits exactly on the last typed character.
                    // Glide lag would make it look like the caret is stuck
                    // behind the cursor after 20-30+ chars of continuous typing.
                    self.pos = target;
                } else {
                    // For navigation jumps (j, k, $, etc.) use smooth glide.
                    let speed = if (diff.y).abs() < 2.0 { 52.0 } else { 34.0 };
                    let k = 1.0 - (-dt * speed).exp();
                    self.pos += diff * k;
                    if (target - self.pos).length() < 0.15 {
                        self.pos = target;
                    }
                }
            }
        } else {
            self.pos = target;
        }
        self.gliding = self.pos != target;

        if typed {
            self.last_type = now;
        }
        self.hue = (self.hue + dt * 0.4) % 1.0;

        // 2. Spawn living effects (active while typing & ambient when resting)
        self.drip_timer += dt;
        let kind = self.kind;
        let w = self.width.clamp(1.0, 10.0);
        match kind {
            CaretKind::Candle => {
                if typed {
                    fire::emit_fire(&mut self.particles, self.pos, w, 1);
                } else if !self.calm && self.drip_timer > 1.4 {
                    self.drip_timer = 0.0;
                    if fastrand::f32() < 0.4 {
                        fire::emit_fire(&mut self.particles, self.pos, w, 1);
                    }
                }
            }
            CaretKind::Fire => {
                if typed {
                    fire::emit_fire(&mut self.particles, self.pos, w, 2);
                } else if !self.calm && self.drip_timer > 0.35 {
                    self.drip_timer = 0.0;
                    fire::emit_fire(&mut self.particles, self.pos, w, 1);
                }
            }
            CaretKind::Water => {
                if typed {
                    self.ripples.push(Ripple {
                        center: eframe::egui::pos2(self.pos.x + w * 0.5, self.pos.y + lh - 1.0),
                        age: 0.0,
                    });
                    water::emit_drops(&mut self.particles, self.pos, w, lh, 2);
                } else if !self.calm && self.drip_timer > 0.75 {
                    self.drip_timer = 0.0;
                    self.ripples.push(Ripple {
                        center: eframe::egui::pos2(self.pos.x + w * 0.5, self.pos.y + lh - 1.0),
                        age: 0.0,
                    });
                    water::emit_drops(&mut self.particles, self.pos, w, lh, 1);
                }
            }
            CaretKind::Snow => {
                if typed {
                    snow::emit_snow(&mut self.particles, self.pos, w, lh, 2);
                } else if !self.calm && self.drip_timer > 0.40 {
                    self.drip_timer = 0.0;
                    snow::emit_snow(&mut self.particles, self.pos, w, lh, 1);
                }
            }
            CaretKind::Electric if typed => {
                if self.bolts.len() < 4 {
                    self.bolts.push(effects::make_bolt(self.pos, cw, lh));
                }
            }
            CaretKind::Comet => {
                self.trail.push_front(self.pos);
                if self.trail.len() > 14 {
                    self.trail.pop_back();
                }
            }
            CaretKind::Matrix => {
                let n = if typed {
                    3
                } else if self.calm {
                    0
                } else if fastrand::f32() < 0.25 {
                    1
                } else {
                    0
                };
                if n > 0 {
                    effects::emit_matrix(&mut self.particles, self.pos, w, n);
                }
            }
            _ => {}
        }

        // 3. Advance particle physics with dt
        for p in &mut self.particles {
            p.age += dt;
            p.pos += p.vel * dt;
            if kind == CaretKind::Water {
                p.vel.y += 220.0 * dt;
            } else if kind == CaretKind::Snow {
                p.vel.x += (fastrand::f32() - 0.5) * 8.0 * dt;
            }
        }
        self.particles.retain(|p| p.age < p.life);
        if self.particles.len() > 200 {
            let extra = self.particles.len() - 200;
            self.particles.drain(0..extra);
        }

        for r in &mut self.ripples {
            r.age += dt;
        }
        self.ripples.retain(|r| r.age < 0.35);

        for b in &mut self.bolts {
            b.age += dt;
        }
        self.bolts.retain(|b| b.age < 0.12);
    }

    fn blink_visible(&self, now: f64) -> bool {
        if !self.blink_enabled {
            return true;
        }
        let idle = now - self.last_type;
        idle < 0.5 || (((idle - 0.5) / 0.53) as u64) % 2 == 0
    }

    fn blink_alpha(&self, now: f64) -> f32 {
        if !self.blink_enabled {
            return 1.0;
        }
        let idle = (now - self.last_type) as f32;
        if idle < 0.45 {
            1.0
        } else {
            let phase = ((idle - 0.45) * std::f32::consts::TAU * 0.95).cos();
            0.5 + 0.5 * phase
        }
    }

    pub fn paint(&self, p: &Painter, cw: f32, lh: f32, now: f64, accent: Color32, is_light: bool) {
        let w = self.width.clamp(1.0, 10.0);

        // Static mode: all animations disabled
        if !self.animations_enabled {
            if !self.blink_visible(now) {
                return;
            }
            match self.kind {
                CaretKind::Block => beam::paint_block(p, self.pos, cw, lh, accent),
                CaretKind::Beam => beam::paint_beam(p, self.pos, w, lh, accent),
                CaretKind::Underline => beam::paint_underline(p, self.pos, cw, w, lh, accent),
                CaretKind::Candle => beam::paint_beam(
                    p,
                    self.pos,
                    w,
                    lh,
                    if is_light {
                        Color32::from_rgb(195, 120, 20)
                    } else {
                        Color32::from_rgb(250, 230, 160)
                    },
                ),
                CaretKind::Fire => {
                    beam::paint_beam(p, self.pos, w, lh, Color32::from_rgb(235, 95, 20))
                }
                CaretKind::Water => beam::paint_beam(
                    p,
                    self.pos,
                    w,
                    lh,
                    if is_light {
                        Color32::from_rgb(20, 130, 225)
                    } else {
                        Color32::from_rgb(65, 175, 255)
                    },
                ),
                CaretKind::Snow => beam::paint_beam(
                    p,
                    self.pos,
                    w,
                    lh,
                    if is_light {
                        Color32::from_rgb(35, 115, 185)
                    } else {
                        Color32::from_rgb(225, 245, 255)
                    },
                ),
                CaretKind::Electric => beam::paint_beam(
                    p,
                    self.pos,
                    w,
                    lh,
                    if is_light {
                        Color32::from_rgb(100, 70, 220)
                    } else {
                        Color32::from_rgb(170, 210, 255)
                    },
                ),
                CaretKind::Matrix => beam::paint_beam(
                    p,
                    self.pos,
                    w,
                    lh,
                    if is_light {
                        Color32::from_rgb(25, 145, 55)
                    } else {
                        Color32::from_rgb(40, 255, 90)
                    },
                ),
                CaretKind::Ice => beam::paint_beam(
                    p,
                    self.pos,
                    w,
                    lh,
                    if is_light {
                        Color32::from_rgb(15, 140, 195)
                    } else {
                        Color32::from_rgb(135, 230, 255)
                    },
                ),
                CaretKind::Heartbeat => {
                    beam::paint_beam(p, self.pos, w, lh, Color32::from_rgb(235, 50, 90))
                }
                _ => beam::paint_beam(p, self.pos, w, lh, accent),
            }
            return;
        }

        let alpha = self.blink_alpha(now);
        if alpha < 0.03 {
            return;
        }

        let with_alpha = |c: Color32| -> Color32 {
            Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), ((c.a() as f32) * alpha) as u8)
        };

        // Animated rendering mode
        match self.kind {
            CaretKind::Block => beam::paint_block(p, self.pos, cw, lh, with_alpha(accent)),
            CaretKind::Beam => beam::paint_beam(p, self.pos, w, lh, with_alpha(accent)),
            CaretKind::Underline => {
                beam::paint_underline(p, self.pos, cw, w, lh, with_alpha(accent))
            }
            CaretKind::Candle => {
                candle::paint_candle(p, self.pos, w, lh, now, &self.particles, is_light)
            }
            CaretKind::Fire => fire::paint_fire(p, self.pos, w, lh, now, &self.particles),
            CaretKind::Water => {
                water::paint_water(p, self.pos, w, lh, &self.ripples, &self.particles)
            }
            CaretKind::Snow => snow::paint_snow(p, self.pos, w, lh, &self.particles, is_light),
            CaretKind::Neon => neon::paint_neon(
                p,
                self.pos,
                w,
                lh,
                if is_light {
                    Color32::from_rgb(25, 150, 60)
                } else {
                    accent
                },
            ),
            CaretKind::Rainbow => rainbow::paint_rainbow(p, self.pos, w, lh, self.hue),
            CaretKind::Electric => effects::paint_electric(p, self.pos, w, lh, &self.bolts),
            CaretKind::Comet => effects::paint_comet(p, self.pos, w, lh, &self.trail, accent),
            CaretKind::Matrix => effects::paint_matrix(p, self.pos, w, lh, &self.particles),
            CaretKind::Ice => effects::paint_ice(p, self.pos, w, lh, now),
            CaretKind::Glitch => effects::paint_glitch(p, self.pos, w, lh, now, self.last_type),
            CaretKind::Heartbeat => effects::paint_heartbeat(p, self.pos, w, lh, now),
        }
    }
}
