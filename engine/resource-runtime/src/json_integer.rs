// SPDX-License-Identifier: MPL-2.0
//! Decode JSON Schema integers by their exact value, including decimal/exponent notation.
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use serde_json::value::RawValue;

fn exact(token: &str) -> Option<i64> {
    // RawValue is syntactically valid JSON. Only number tokens are admissible.
    let unsigned = token.strip_prefix('-').unwrap_or(token);
    if !unsigned.as_bytes().first()?.is_ascii_digit() {
        return None;
    }
    let (mantissa, exponent) = unsigned.split_once(['e', 'E']).unwrap_or((unsigned, "0"));
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{whole}{fraction}");
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return Some(0);
    }

    let significant = digits.trim_end_matches('0');
    let zeros = i64::try_from(digits.len() - significant.len()).ok()?;
    let scale = exponent
        .parse::<i64>()
        .ok()?
        .checked_sub(i64::try_from(fraction.len()).ok()?)?
        .checked_add(zeros)?;
    // i32 and u32 have at most ten decimal digits. A negative scale leaves a fraction.
    if !(0..=9).contains(&scale) || significant.len() + scale as usize > 10 {
        return None;
    }
    let magnitude = significant.parse::<i64>().ok()? * 10_i64.pow(scale as u32);
    Some(if token.starts_with('-') {
        -magnitude
    } else {
        magnitude
    })
}

fn convert<E: Error, T: TryFrom<i64>>(raw: &RawValue) -> Result<T, E> {
    exact(raw.get())
        .and_then(|n| T::try_from(n).ok())
        .ok_or_else(|| E::custom(format!("expected an in-range integer, got {}", raw.get())))
}

/// Decode a JSON integer exactly into the requested 32-bit numeric type.
pub fn read<'de, D: Deserializer<'de>, T: TryFrom<i64>>(de: D) -> Result<T, D::Error> {
    convert(<&RawValue>::deserialize(de)?)
}

/// Decode a present integer field. Pair with serde(default) for absent fields.
pub fn optional<'de, D: Deserializer<'de>, T: TryFrom<i64>>(de: D) -> Result<Option<T>, D::Error> {
    read(de).map(Some)
}
