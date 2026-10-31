pub(super) fn parse(text: &str) -> Option<isize> {
    let text = text.replace('_', "");
    let (negative, text) = text
        .strip_prefix('-')
        .map_or((false, text.as_str()), |text| (true, text));
    let (radix, digits) = radix(text);
    let value = isize::from_str_radix(digits, radix).ok()?;
    if negative {
        value.checked_neg()
    } else {
        Some(value)
    }
}
fn radix(text: &str) -> (u32, &str) {
    match text.get(..2).unwrap_or("") {
        "0x" | "0X" => (16, &text[2..]),
        "0b" | "0B" => (2, &text[2..]),
        "0o" | "0O" => (8, &text[2..]),
        "0d" | "0D" => (10, &text[2..]),
        _ if text.len() > 1 && text.starts_with('0') => (8, text),
        _ => (10, text),
    }
}
