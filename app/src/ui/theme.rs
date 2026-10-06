//! Theme palette definitions for MINDFORGE's 19 visual themes.
//!
//! Every theme's text/accent/muted/highlight colors are checked against its
//! background at test time using a real WCAG contrast-ratio calculation
//! (see `contrast_ratio` + the `readability` test module at the bottom).
//! Adding a theme with poor contrast fails `cargo test` instead of shipping
//! something hard to read.
//!
//! Each theme provides a complete `Theme` struct with all colors needed by
//! every UI component — no hardcoded colors exist outside this module.

use eframe::egui::Color32;

/// Enumeration of all 19 available color themes.
///
/// Use `ThemeKind::ALL` to iterate all themes, `display_name()` for UI labels,
/// and `name()` for the lowercase identifier used in settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemeKind {
    Shell,
    Cream,
    Latte,
    White,
    TokyoNight,
    Dracula,
    Catppuccin,
    Nord,
    RosePine,
    Cyberpunk,
    Green,
    Amber,
    Ice,
    Gruvbox,
    Everforest,
    Monokai,
    OneDark,
    Ayu,
    Kanagawa,
}

impl ThemeKind {
    pub const ALL: &'static [ThemeKind] = &[
        ThemeKind::Cream,
        ThemeKind::Latte,
        ThemeKind::White,
        ThemeKind::Shell,
        ThemeKind::TokyoNight,
        ThemeKind::Dracula,
        ThemeKind::Catppuccin,
        ThemeKind::Nord,
        ThemeKind::RosePine,
        ThemeKind::Cyberpunk,
        ThemeKind::Green,
        ThemeKind::Amber,
        ThemeKind::Ice,
        ThemeKind::Gruvbox,
        ThemeKind::Everforest,
        ThemeKind::Monokai,
        ThemeKind::OneDark,
        ThemeKind::Ayu,
        ThemeKind::Kanagawa,
    ];

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().replace(['-', '_', ' '], "").as_str() {
            "cream" | "warmcream" | "parchment" | "papercream" => Some(Self::Cream),
            "latte" | "catppuccinlatte" | "milky" => Some(Self::Latte),
            "white" | "light" | "purelight" | "paper" | "mono" | "monochrome" => Some(Self::White),
            "shell" | "012b36" | "terminal" | "solarized" | "solarizeddark" => Some(Self::Shell),
            "green" | "matrix" => Some(Self::Green),
            "amber" => Some(Self::Amber),
            "tokyo" | "tokyonight" => Some(Self::TokyoNight),
            "dracula" => Some(Self::Dracula),
            "catppuccin" | "mocha" => Some(Self::Catppuccin),
            "nord" => Some(Self::Nord),
            "cyberpunk" | "cyber" => Some(Self::Cyberpunk),
            "rose" | "rosepine" => Some(Self::RosePine),
            "ice" => Some(Self::Ice),
            "gruvbox" | "gruv" => Some(Self::Gruvbox),
            "everforest" | "forest" => Some(Self::Everforest),
            "monokai" | "monokaipro" => Some(Self::Monokai),
            "onedark" | "atom" => Some(Self::OneDark),
            "ayu" | "ayudark" => Some(Self::Ayu),
            "kanagawa" => Some(Self::Kanagawa),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Cream => "cream",
            Self::Latte => "latte",
            Self::White => "white",
            Self::Shell => "shell",
            Self::TokyoNight => "tokyonight",
            Self::Dracula => "dracula",
            Self::Catppuccin => "catppuccin",
            Self::Nord => "nord",
            Self::RosePine => "rosepine",
            Self::Cyberpunk => "cyberpunk",
            Self::Green => "green",
            Self::Amber => "amber",
            Self::Ice => "ice",
            Self::Gruvbox => "gruvbox",
            Self::Everforest => "everforest",
            Self::Monokai => "monokai",
            Self::OneDark => "onedark",
            Self::Ayu => "ayu",
            Self::Kanagawa => "kanagawa",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Cream => "Warm Cream",
            Self::Latte => "Catppuccin Latte",
            Self::White => "Pure Light",
            Self::Shell => "Shell (#012b36)",
            Self::TokyoNight => "Tokyo Night",
            Self::Dracula => "Dracula",
            Self::Catppuccin => "Catppuccin",
            Self::Nord => "Nord Arctic",
            Self::RosePine => "Rosé Pine",
            Self::Cyberpunk => "Cyberpunk",
            Self::Green => "Matrix Green",
            Self::Amber => "Amber CRT",
            Self::Ice => "Glacier Ice",
            Self::Gruvbox => "Gruvbox Dark",
            Self::Everforest => "Everforest",
            Self::Monokai => "Monokai Pro",
            Self::OneDark => "One Dark",
            Self::Ayu => "Ayu Dark",
            Self::Kanagawa => "Kanagawa",
        }
    }

    pub fn is_light(&self) -> bool {
        matches!(self, Self::Cream | Self::Latte | Self::White)
    }
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub kind: ThemeKind,
    pub bg: Color32,
    pub text: Color32,
    pub accent: Color32,
    pub muted: Color32,
    pub highlight: Color32,
}

impl Theme {
    pub fn is_light(&self) -> bool {
        self.kind.is_light() || relative_luminance(self.bg) > 0.5
    }

    /// Slightly elevated surface color for cards and active tabs
    pub fn surface(&self) -> Color32 {
        if self.is_light() {
            let r = self.bg.r().saturating_sub(10);
            let g = self.bg.g().saturating_sub(10);
            let b = self.bg.b().saturating_sub(12);
            Color32::from_rgb(r, g, b)
        } else {
            let r = (self.bg.r() as u16 + 14).min(255) as u8;
            let g = (self.bg.g() as u16 + 16).min(255) as u8;
            let b = (self.bg.b() as u16 + 22).min(255) as u8;
            Color32::from_rgb(r, g, b)
        }
    }

    /// Border color for containers and dividers
    pub fn border(&self) -> Color32 {
        if self.is_light() {
            let r = self.bg.r().saturating_sub(30);
            let g = self.bg.g().saturating_sub(30);
            let b = self.bg.b().saturating_sub(34);
            Color32::from_rgb(r, g, b)
        } else {
            let r = (self.bg.r() as u16 + 28).min(255) as u8;
            let g = (self.bg.g() as u16 + 32).min(255) as u8;
            let b = (self.bg.b() as u16 + 40).min(255) as u8;
            Color32::from_rgb(r, g, b)
        }
    }

    /// Sidebar background color (subtly adjusted relative to theme.bg)
    pub fn sidebar_bg(&self) -> Color32 {
        if self.is_light() {
            let r = self.bg.r().saturating_sub(5);
            let g = self.bg.g().saturating_sub(5);
            let b = self.bg.b().saturating_sub(7);
            Color32::from_rgb(r, g, b)
        } else {
            let r = self.bg.r().saturating_sub(4);
            let g = self.bg.g().saturating_sub(4);
            let b = self.bg.b().saturating_sub(4);
            Color32::from_rgb(r, g, b)
        }
    }

    pub fn from_kind(kind: ThemeKind) -> Self {
        match kind {
            ThemeKind::Shell => Self {
                kind,
                bg: Color32::from_rgb(0, 36, 44),
                text: Color32::from_rgb(246, 242, 226),
                accent: Color32::from_rgb(42, 192, 180),
                muted: Color32::from_rgb(128, 154, 162),
                highlight: Color32::from_rgb(45, 165, 245),
            },
            ThemeKind::TokyoNight => Self {
                kind,
                bg: Color32::from_rgb(22, 23, 34),
                text: Color32::from_rgb(212, 222, 255),
                accent: Color32::from_rgb(122, 162, 247),
                muted: Color32::from_rgb(138, 148, 188),
                highlight: Color32::from_rgb(187, 154, 247),
            },
            ThemeKind::Dracula => Self {
                kind,
                bg: Color32::from_rgb(34, 36, 48),
                text: Color32::from_rgb(248, 248, 242),
                accent: Color32::from_rgb(255, 121, 198),
                muted: Color32::from_rgb(155, 162, 200),
                highlight: Color32::from_rgb(139, 233, 253),
            },
            ThemeKind::Catppuccin => Self {
                kind,
                bg: Color32::from_rgb(24, 24, 37),
                text: Color32::from_rgb(215, 222, 248),
                accent: Color32::from_rgb(203, 166, 247),
                muted: Color32::from_rgb(152, 158, 188),
                highlight: Color32::from_rgb(245, 194, 231),
            },
            ThemeKind::Nord => Self {
                kind,
                bg: Color32::from_rgb(40, 46, 58),
                text: Color32::from_rgb(240, 243, 248),
                accent: Color32::from_rgb(136, 192, 208),
                muted: Color32::from_rgb(155, 165, 190),
                highlight: Color32::from_rgb(143, 188, 187),
            },
            ThemeKind::RosePine => Self {
                kind,
                bg: Color32::from_rgb(25, 23, 36),
                text: Color32::from_rgb(235, 233, 250),
                accent: Color32::from_rgb(235, 111, 146),
                muted: Color32::from_rgb(155, 150, 180),
                highlight: Color32::from_rgb(246, 193, 119),
            },
            ThemeKind::Cyberpunk => Self {
                kind,
                bg: Color32::from_rgb(18, 13, 30),
                text: Color32::from_rgb(180, 250, 255),
                accent: Color32::from_rgb(255, 225, 53),
                muted: Color32::from_rgb(165, 130, 195),
                highlight: Color32::from_rgb(255, 42, 109),
            },
            ThemeKind::Green => Self {
                kind,
                bg: Color32::from_rgb(8, 18, 10),
                text: Color32::from_rgb(200, 255, 215),
                accent: Color32::from_rgb(51, 255, 102),
                muted: Color32::from_rgb(115, 175, 135),
                highlight: Color32::from_rgb(130, 255, 180),
            },
            ThemeKind::Amber => Self {
                kind,
                bg: Color32::from_rgb(18, 14, 8),
                text: Color32::from_rgb(255, 235, 195),
                accent: Color32::from_rgb(255, 175, 45),
                muted: Color32::from_rgb(185, 145, 95),
                highlight: Color32::from_rgb(255, 215, 105),
            },
            ThemeKind::Ice => Self {
                kind,
                bg: Color32::from_rgb(10, 20, 32),
                text: Color32::from_rgb(225, 248, 255),
                accent: Color32::from_rgb(80, 210, 255),
                muted: Color32::from_rgb(115, 175, 205),
                highlight: Color32::from_rgb(160, 240, 255),
            },
            ThemeKind::Cream => Self {
                kind,
                bg: Color32::from_rgb(247, 243, 233), // #f7f3e9 warm parchment cream
                text: Color32::from_rgb(44, 38, 33),  // #2c2621 deep espresso ink (AAA contrast)
                accent: Color32::from_rgb(168, 70, 22), // #a84616 warm terracotta ember
                muted: Color32::from_rgb(118, 108, 96), // #766c60 warm stone gray
                highlight: Color32::from_rgb(28, 102, 118), // #1c6676 deep jade teal
            },
            ThemeKind::Latte => Self {
                kind,
                bg: Color32::from_rgb(239, 241, 245), // #eff1f5 catppuccin latte base
                text: Color32::from_rgb(76, 79, 105), // #4c4f69 deep slate ink
                accent: Color32::from_rgb(136, 57, 239), // #8839ef vivid lavender
                muted: Color32::from_rgb(112, 115, 134), // #707386 soft graphite
                highlight: Color32::from_rgb(30, 102, 245), // #1e66f5 sapphire blue
            },
            ThemeKind::White => Self {
                kind,
                bg: Color32::from_rgb(250, 250, 252), // #fafafc crisp paper white
                text: Color32::from_rgb(28, 31, 35),  // #1c1f23 deep slate ink (AAA contrast)
                accent: Color32::from_rgb(9, 105, 218), // #0969da modern electric blue
                muted: Color32::from_rgb(101, 109, 118), // #656d76 neutral slate gray
                highlight: Color32::from_rgb(110, 84, 148), // #6e5494 royal indigo
            },

            // --- New themes ---
            ThemeKind::Gruvbox => Self {
                kind,
                bg: Color32::from_rgb(32, 32, 32),
                text: Color32::from_rgb(240, 224, 185),
                accent: Color32::from_rgb(254, 128, 25),
                muted: Color32::from_rgb(175, 160, 140),
                highlight: Color32::from_rgb(184, 187, 38),
            },
            ThemeKind::Everforest => Self {
                kind,
                bg: Color32::from_rgb(31, 38, 42),
                text: Color32::from_rgb(218, 206, 180),
                accent: Color32::from_rgb(167, 192, 128),
                muted: Color32::from_rgb(155, 165, 152),
                highlight: Color32::from_rgb(127, 187, 179),
            },
            ThemeKind::Monokai => Self {
                kind,
                bg: Color32::from_rgb(36, 33, 37),
                text: Color32::from_rgb(253, 253, 251),
                accent: Color32::from_rgb(255, 216, 102),
                muted: Color32::from_rgb(170, 165, 172),
                highlight: Color32::from_rgb(255, 97, 136),
            },
            ThemeKind::OneDark => Self {
                kind,
                bg: Color32::from_rgb(33, 37, 43),
                text: Color32::from_rgb(228, 231, 236),
                accent: Color32::from_rgb(97, 175, 239),
                muted: Color32::from_rgb(158, 166, 182),
                highlight: Color32::from_rgb(198, 120, 221),
            },
            ThemeKind::Ayu => Self {
                kind,
                bg: Color32::from_rgb(8, 12, 18),
                text: Color32::from_rgb(205, 203, 196),
                accent: Color32::from_rgb(255, 180, 84),
                muted: Color32::from_rgb(125, 137, 155),
                highlight: Color32::from_rgb(89, 194, 255),
            },
            ThemeKind::Kanagawa => Self {
                kind,
                bg: Color32::from_rgb(24, 24, 32),
                text: Color32::from_rgb(228, 223, 196),
                accent: Color32::from_rgb(126, 156, 216),
                muted: Color32::from_rgb(156, 153, 140),
                highlight: Color32::from_rgb(210, 126, 153),
            },
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_kind(ThemeKind::Shell)
    }
}

/// WCAG relative luminance of an sRGB color (0.0 = black, 1.0 = white).
/// https://www.w3.org/TR/WCAG21/#dfn-relative-luminance
pub fn relative_luminance(c: Color32) -> f32 {
    let chan = |v: u8| -> f32 {
        let s = v as f32 / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * chan(c.r()) + 0.7152 * chan(c.g()) + 0.0722 * chan(c.b())
}

/// WCAG contrast ratio between two colors, from 1.0 (no contrast) to 21.0
/// (black on white). 4.5 is the AA threshold for normal text; 3.0 is the AA
/// threshold for large text and UI components. Order of arguments doesn't
/// matter — the lighter color is detected automatically.
pub fn contrast_ratio(a: Color32, b: Color32) -> f32 {
    let (l1, l2) = (relative_luminance(a), relative_luminance(b));
    let (lighter, darker) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

#[cfg(test)]
mod readability {
    use super::*;

    /// Body text must clear the WCAG AA threshold for normal text against
    /// the background. This is the color people spend the most time reading.
    const MIN_TEXT_CONTRAST: f32 = 4.5;
    /// Accent/highlight/muted are decorative or used sparingly (labels,
    /// borders, small UI bits) — WCAG's lower "large text / UI component"
    /// threshold applies.
    const MIN_UI_CONTRAST: f32 = 3.0;

    #[test]
    fn every_theme_is_readable() {
        for &kind in ThemeKind::ALL {
            let t = Theme::from_kind(kind);
            let text_c = contrast_ratio(t.text, t.bg);
            assert!(
                text_c >= MIN_TEXT_CONTRAST,
                "{}: text vs bg contrast {:.2} is below {} (text is hard to read)",
                kind.name(),
                text_c,
                MIN_TEXT_CONTRAST
            );

            for (label, color) in [
                ("accent", t.accent),
                ("muted", t.muted),
                ("highlight", t.highlight),
            ] {
                let c = contrast_ratio(color, t.bg);
                assert!(
                    c >= MIN_UI_CONTRAST,
                    "{}: {} vs bg contrast {:.2} is below {}",
                    kind.name(),
                    label,
                    c,
                    MIN_UI_CONTRAST
                );
            }
        }
    }

    #[test]
    fn contrast_ratio_is_symmetric_and_sane() {
        assert!((contrast_ratio(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 0.1);
        assert!((contrast_ratio(Color32::WHITE, Color32::BLACK) - 21.0).abs() < 0.1);
        assert!((contrast_ratio(Color32::WHITE, Color32::WHITE) - 1.0).abs() < 0.01);
    }

    #[test]
    fn parse_round_trips_for_all_themes() {
        for &kind in ThemeKind::ALL {
            assert_eq!(
                ThemeKind::parse(kind.name()),
                Some(kind),
                "name() -> parse() round trip failed for {kind:?}"
            );
        }
    }
}
