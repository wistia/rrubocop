use num_bigint::BigInt;
pub(super) fn parse(text: &str) -> Option<BigInt> {
    let text = text.replace('_', "");
    let (radix, digits) = digits(text.trim_start_matches(['-', '+']));
    let number = BigInt::parse_bytes(digits.as_bytes(), radix)?;
    Some(if text.starts_with('-') {
        -number
    } else {
        number
    })
}
fn digits(text: &str) -> (u32, &str) {
    match text.get(..2).unwrap_or("") {
        "0x" | "0X" => (16, &text[2..]),
        "0o" | "0O" => (8, &text[2..]),
        "0b" | "0B" => (2, &text[2..]),
        "0d" | "0D" => (10, &text[2..]),
        _ if text.len() > 1 && text.starts_with('0') => (8, text),
        _ => (10, text),
    }
}
