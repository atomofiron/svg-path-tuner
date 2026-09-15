//! Fixed point decimal coordinates.
//!
//! Every coordinate is multiplied by `10^DECIMALS` and kept as an integer, so all the path
//! math (accumulation, deltas, scaling) is exact integer math and rounding happens exactly
//! once per input value instead of on every floating point operation.

use crate::ext::Rslt;

/// A coordinate scaled by `10^PRECISION`.
pub type Fixed = i64;

/// Digits kept after the decimal point in the output: `960 * 10^4` is the largest product of a
/// coordinate below 960 and a power of ten whose integer part stays exact in `f32`.
const DECIMALS: u32 = 4;

/// Digits kept internally, more than the output precision, so scaling never double rounds.
const PRECISION: u32 = 8;

/// A coordinate of `1.0`.
const UNIT: Fixed = 10_i64.pow(PRECISION);

/// Parses `12`, `-4.5`, `.5`, `1.5e2` into fixed point, rounding away the digits past `PRECISION`.
pub fn parse(token: &str) -> Rslt<Fixed> {
    let invalid = || format!("{token} is not a number");

    let (negative, token) = match token.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, token.strip_prefix('+').unwrap_or(token)),
    };
    let (mantissa, exponent) = match token.find(['e', 'E']) {
        Some(position) => (
            &token[..position],
            token[position + 1..].parse::<i32>().map_err(|_| invalid())?,
        ),
        None => (token, 0),
    };
    let (integer, fraction) = match mantissa.find('.') {
        Some(position) => (&mantissa[..position], &mantissa[position + 1..]),
        None => (mantissa, ""),
    };
    if integer.is_empty() && fraction.is_empty() {
        return Err(invalid().into());
    }
    if !integer.bytes().all(|b| b.is_ascii_digit()) || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid().into());
    }

    let digits = format!("{integer}{fraction}");
    let value = digits.parse::<i128>().map_err(|_| invalid())?;
    let power = exponent - fraction.len() as i32 + PRECISION as i32;
    let value = if power >= 0 {
        let factor = power_of_ten(power as u32).ok_or_else(|| format!("{token} is too large"))?;
        value
            .checked_mul(factor)
            .ok_or_else(|| format!("{token} is too large"))?
    } else {
        let divisor = power_of_ten(-power as u32).ok_or_else(|| format!("{token} is too small"))?;
        round_divide(value, divisor)
    };
    let value = if negative { -value } else { value };

    i64::try_from(value).map_err(|_| format!("{token} is too large").into())
}

/// Formats fixed point as a plain decimal rounded to `DECIMALS`, exact and float free.
pub fn format(value: Fixed) -> String {
    let quantum = 10_i64.pow(PRECISION - DECIMALS);
    let value = round_divide(value as i128, quantum as i128);
    let absolute = value.unsigned_abs();
    let integer = absolute / OUTPUT_UNIT as u128;
    let fraction = absolute % OUTPUT_UNIT as u128;
    let sign = if value < 0 { "-" } else { "" };
    if fraction == 0 {
        return format!("{sign}{integer}");
    }
    let mut digits = format!("{fraction:0width$}", width = DECIMALS as usize);
    while digits.ends_with('0') {
        digits.pop();
    }
    format!("{sign}{integer}.{digits}")
}

/// Formats the ratio `numerator / denominator` in the output precision.
pub fn format_ratio(numerator: i64, denominator: i64) -> String {
    match scaled(UNIT, numerator, denominator) {
        Some(value) => format(value),
        None => format!("{numerator}/{denominator}"),
    }
}

/// Multiplies by `numerator / denominator`, `None` when the result leaves the fixed point range.
pub fn scaled(value: Fixed, numerator: i64, denominator: i64) -> Option<Fixed> {
    let value = value as i128 * numerator as i128;
    i64::try_from(round_divide(value, denominator as i128)).ok()
}

const OUTPUT_UNIT: i64 = 10_i64.pow(DECIMALS);

fn power_of_ten(power: u32) -> Option<i128> {
    10_i128.checked_pow(power)
}

fn round_divide(value: i128, divisor: i128) -> i128 {
    let half = divisor / 2;
    if value >= 0 {
        (value + half) / divisor
    } else {
        -((-value + half) / divisor)
    }
}
