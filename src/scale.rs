use std::fmt::{Display, Formatter};

use crate::fixed::{self, Fixed};
use crate::size::Size;

/// Ratio of one axis, kept as a fraction so scaling stays exact integer math.
#[derive(Clone, Copy)]
pub struct Ratio {
    numerator: Fixed,
    denominator: Fixed,
}

impl Ratio {
    fn new(to: Fixed, from: Fixed) -> Ratio {
        Ratio { numerator: to, denominator: from }
    }

    fn identity() -> Ratio {
        Ratio { numerator: 1, denominator: 1 }
    }

    pub fn apply(&self, value: Fixed) -> Option<Fixed> {
        fixed::scaled(value, self.numerator, self.denominator)
    }
}

/// Ratios of both axes, computed from the file viewport and the target size.
#[derive(Clone, Copy)]
pub struct Scale {
    pub x: Ratio,
    pub y: Ratio,
}

impl Scale {
    pub fn identity() -> Scale {
        Scale { x: Ratio::identity(), y: Ratio::identity() }
    }

    /// Ratios that turn the `viewport` into the target `size`, no `size` means no scaling.
    pub fn fit(viewport: Option<(Fixed, Fixed)>, size: Option<Size>) -> Result<Scale, String> {
        let Some(size) = size else {
            return Ok(Scale::identity());
        };
        let Some((width, height)) = viewport else {
            return Err("no viewport to compute the size against".to_owned());
        };
        if width <= 0 || height <= 0 {
            return Err("the viewport is not greater than 0".to_owned());
        }
        Ok(Scale { x: Ratio::new(size.width, width), y: Ratio::new(size.height, height) })
    }
}

impl Display for Ratio {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", fixed::format_ratio(self.numerator, self.denominator))
    }
}

impl Display for Scale {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.x, self.y)
    }
}
