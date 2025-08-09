#![no_std]

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black,
    DarkGray,
    Red,
    LightRed,
    Green,
    LightGreen,
    Yellow,
    LightYellow,
    Blue,
    LightBlue,
    Purple,
    LightPurple,
    Magenta,
    LightMagenta,
    Cyan,
    LightCyan,
    White,
    LightGray,
    Default,
}

impl Color {
    fn code(self) -> (u8, bool) {
        match self {
            Self::Black => (0, false),
            Self::DarkGray => (0, true),
            Self::Red => (1, false),
            Self::LightRed => (1, true),
            Self::Green => (2, false),
            Self::LightGreen => (2, true),
            Self::Yellow => (3, false),
            Self::LightYellow => (3, true),
            Self::Blue => (4, false),
            Self::LightBlue => (4, true),
            Self::Purple | Self::Magenta => (5, false),
            Self::LightPurple | Self::LightMagenta => (5, true),
            Self::Cyan => (6, false),
            Self::LightCyan => (6, true),
            Self::White => (7, false),
            Self::LightGray => (7, true),
            Self::Default => (9, false),
        }
    }

    fn fg(self) -> u8 {
        let (code, light) = self.code();
        if light { code + 90 } else { code + 30 }
    }
}

pub struct WithFg<T>(Color, T);

impl<T> fmt::Display for WithFg<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fg = self.0.fg();
        let value = &self.1;
        write!(f, "\x1B[{fg};1m{value}\x1B[0m")
    }
}

impl<T> WithFg<T> {
    pub fn new(color: Color, value: T) -> Self {
        Self(color, value)
    }
}
