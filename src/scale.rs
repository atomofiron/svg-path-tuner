use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::fixed::{self, Fixed};

/// Scale ratio of the `-s` argument, kept as a fraction so scaling stays exact integer math.
#[derive(Clone, Copy)]
pub struct Scale {
    numerator: i64,
    denominator: i64,
}

impl Scale {
    fn new(numerator: i64, denominator: i64) -> Result<Scale, String> {
        if numerator <= 0 || denominator <= 0 {
            return Err("scale must be greater than 0".to_owned());
        }
        Ok(Scale { numerator, denominator })
    }

    pub fn apply(&self, value: Fixed) -> Fixed {
        fixed::scaled(value, self.numerator, self.denominator)
    }
}

impl FromStr for Scale {
    type Err = String;

    fn from_str(value: &str) -> Result<Scale, String> {
        let (divide, digits) = match value.chars().nth(0) {
            Some('/') => (true, &value[1..]),
            _ => (false, value),
        };
        let number = digits
            .parse::<i64>()
            .map_err(|_| format!("{value} is not a number"))?;
        if divide {
            Scale::new(1, number)
        } else {
            Scale::new(number, 1)
        }
    }
}

impl Display for Scale {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", fixed::format_ratio(self.numerator, self.denominator))
    }
}
