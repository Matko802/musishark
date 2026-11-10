use gpui::{Rgba, rgb};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    Amoled,
    White,
}

impl ThemeMode {
    pub fn label(self) -> &'static str {
        match self {
            ThemeMode::Amoled => "AMOLED",
            ThemeMode::White => "White",
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
}
