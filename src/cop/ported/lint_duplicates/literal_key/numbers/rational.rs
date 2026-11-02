use super::*;

pub(super) fn key(c: &Context<'_>, n: Node<'_>, negative: bool) -> Option<String> {
    let (numerator, denominator) = parts(c, n)?;
    let divisor = gcd(numerator.clone(), denominator.clone());
    let numerator = numerator / &divisor;
    Some(format!(
        "rational:{}/{}",
        if negative { -numerator } else { numerator },
        denominator / divisor
    ))
}
fn parts(c: &Context<'_>, n: Node<'_>) -> Option<(BigInt, BigInt)> {
    let literal = n.named_child(0)?;
    let text = c.text(literal).replace('_', "");
    if literal.kind() == "integer" {
        Some((integer(&text)?, BigInt::from(1)))
    } else {
        decimal(&text)
    }
}

fn decimal(text: &str) -> Option<(BigInt, BigInt)> {
    let (mantissa, exponent) = exponent(text)?;
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let scale = scale(exponent, fraction.len())?;
    let value = BigInt::parse_bytes(format!("{whole}{fraction}").as_bytes(), 10)?;
    Some(scaled(value, scale))
}
fn scale(exponent: i32, fraction: usize) -> Option<i32> {
    let scale = exponent.checked_sub(i32::try_from(fraction).ok()?)?;
    (scale.unsigned_abs() <= 4096).then_some(scale)
}
fn scaled(value: BigInt, scale: i32) -> (BigInt, BigInt) {
    let power = BigInt::from(10).pow(scale.unsigned_abs());
    if scale < 0 {
        (value, power)
    } else {
        (value * power, BigInt::from(1))
    }
}

fn exponent(text: &str) -> Option<(&str, i32)> {
    match text.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => Some((mantissa, exponent.parse().ok()?)),
        None => Some((text, 0)),
    }
}
fn gcd(mut left: BigInt, mut right: BigInt) -> BigInt {
    while right != BigInt::from(0) {
        let remainder = left % &right;
        left = right;
        right = remainder;
    }
    left
}
