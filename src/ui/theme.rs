use gpui::{Rgba, rgb};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    Amoled,
    White,
}

impl ThemeMode {
    pub fn key(self) -> &'static str {
        match self {
            ThemeMode::Amoled => "amoled",
            ThemeMode::White => "white",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ThemeMode::Amoled => "AMOLED",
            ThemeMode::White => "White",
        }
    }

    pub fn from_pref(value: Option<&str>) -> Self {
        match value {
            Some("white") | Some("light") => ThemeMode::White,
            _ => ThemeMode::Amoled,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub mode: ThemeMode,
    pub dark: bool,
    pub bg: Rgba,
    pub surface: Rgba,
    pub surface_hover: Rgba,
    pub elevated: Rgba,
    pub border: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    pub accent: Rgba,
    pub on_accent: Rgba,
    pub danger: Rgba,
}

impl Theme {
    pub fn new(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Amoled => Self {
                mode,
                dark: true,
                bg: rgb(0x000000),
                surface: rgb(0x0c0c0c),
                surface_hover: rgb(0x1a1a1a),
                elevated: rgb(0x151515),
                border: rgb(0x242424),
                text: rgb(0xf7f7f7),
                muted: rgb(0x9b9b9b),
                accent: rgb(0xff453a),
                on_accent: rgb(0x000000),
                danger: rgb(0xff6b6b),
            },
            ThemeMode::White => Self {
                mode,
                dark: false,
                bg: rgb(0xffffff),
                surface: rgb(0xf6f6f6),
                surface_hover: rgb(0xececec),
                elevated: rgb(0xfbfbfb),
                border: rgb(0xdedede),
                text: rgb(0x111111),
                muted: rgb(0x6b6b6b),
                accent: rgb(0xd70015),
                on_accent: rgb(0xffffff),
                danger: rgb(0xc62828),
            },
        }
    }

    pub fn lerp(from: Self, to: Self, t: f32) -> Self {
        fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
            Rgba { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t, a: a.a + (b.a - a.a) * t }
        }
        Self {
            mode: to.mode,
            dark: to.dark,
            bg: mix(from.bg, to.bg, t),
            surface: mix(from.surface, to.surface, t),
            surface_hover: mix(from.surface_hover, to.surface_hover, t),
            elevated: mix(from.elevated, to.elevated, t),
            border: mix(from.border, to.border, t),
            text: mix(from.text, to.text, t),
            muted: mix(from.muted, to.muted, t),
            accent: mix(from.accent, to.accent, t),
            on_accent: mix(from.on_accent, to.on_accent, t),
            danger: mix(from.danger, to.danger, t),
        }
    }
}
