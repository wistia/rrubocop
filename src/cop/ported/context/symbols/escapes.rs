pub(super) fn append(
    bytes: &mut std::iter::Peekable<std::str::Bytes<'_>>,
    result: &mut Vec<u8>,
) -> Option<()> {
    let byte = bytes.next()?;
    match byte {
        b'x' => result.push(digits(bytes, 16, 2)? as u8),
        b'0'..=b'7' => result.push(octal(byte, bytes)),
        b'u' => unicode(bytes, result)?,
        b'\n' | b'\r' => newline(byte, bytes, result),
        b'c' | b'C' | b'M' => control::append(byte, bytes, result)?,
        _ => result.push(simple(byte)),
    }
    Some(())
}
fn digits(
    bytes: &mut std::iter::Peekable<std::str::Bytes<'_>>,
    radix: u32,
    limit: usize,
) -> Option<u32> {
    let mut value = None;
    for _ in 0..limit {
        let Some(digit) = bytes.peek().and_then(|b| (*b as char).to_digit(radix)) else {
            break;
        };
        bytes.next();
        value = Some(value.unwrap_or(0) * radix + digit);
    }
    value
}
fn unicode(
    bytes: &mut std::iter::Peekable<std::str::Bytes<'_>>,
    result: &mut Vec<u8>,
) -> Option<()> {
    if bytes.peek() != Some(&b'{') {
        return codepoint(digits(bytes, 16, 4)?, result);
    }
    bytes.next();
    unicode_list(bytes, result)
}
fn unicode_list(
    bytes: &mut std::iter::Peekable<std::str::Bytes<'_>>,
    result: &mut Vec<u8>,
) -> Option<()> {
    loop {
        while bytes.peek().is_some_and(u8::is_ascii_whitespace) {
            bytes.next();
        }
        if bytes.peek() == Some(&b'}') {
            bytes.next();
            return Some(());
        }
        codepoint(digits(bytes, 16, 6)?, result)?;
    }
}
fn codepoint(value: u32, result: &mut Vec<u8>) -> Option<()> {
    result.extend_from_slice(char::from_u32(value)?.encode_utf8(&mut [0; 4]).as_bytes());
    Some(())
}

fn octal(first: u8, bytes: &mut std::iter::Peekable<std::str::Bytes<'_>>) -> u8 {
    let mut value = first - b'0';
    for _ in 0..2 {
        let Some(digit) = bytes.peek().and_then(|b| (*b as char).to_digit(8)) else {
            break;
        };
        bytes.next();
        value = value.wrapping_mul(8).wrapping_add(digit as u8);
    }
    value
}

fn simple(byte: u8) -> u8 {
    b"nrtvfbaes"
        .iter()
        .position(|b| *b == byte)
        .map_or(byte, |index| {
            [b'\n', b'\r', b'\t', 11, 12, 8, 7, 27, b' '][index]
        })
}

fn newline(byte: u8, bytes: &mut std::iter::Peekable<std::str::Bytes<'_>>, result: &mut Vec<u8>) {
    if byte == b'\r' {
        if bytes.peek() == Some(&b'\n') {
            bytes.next();
        } else {
            result.push(byte);
        }
    }
}

mod control;
