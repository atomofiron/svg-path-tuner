use std::str::FromStr;

use crate::fixed::{self, Fixed};

/// Target viewport size of the `-s` argument: `12` means `12x12`, `24x24` sets both axes.
#[derive(Clone, Copy)]
pub struct Size {
    pub width: Fixed,
    pub height: Fixed,
}

impl FromStr for Size {
    type Err = String;

    fn from_str(value: &str) -> Result<Size, String> {
        let (width, height) = match value.split_once(['x', 'X']) {
            Some((width, height)) => (width, height),
            None => (value, value),
        };
        Ok(Size { width: side(width, value)?, height: side(height, value)? })
    }
}

fn side(side: &str, value: &str) -> Result<Fixed, String> {
    let error = || format!("{value} is not a size, expected W or WxH");
    let size = fixed::parse(side.trim()).map_err(|_| error())?;
    if size <= 0 {
        return Err(error());
    }
    Ok(size)
}
