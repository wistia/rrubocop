enum Operand {
    Byte(u8),
    Modifier(u8),
}

pub(super) fn append(
    mut modifier: u8,
    bytes: &mut std::iter::Peekable<std::str::Bytes<'_>>,
    result: &mut Vec<u8>,
) -> Option<()> {
    let mut modifiers = Vec::new();
    loop {
        modifiers.push(modifier);
        match operand(modifier, bytes)? {
            Operand::Modifier(next) => modifier = next,
            Operand::Byte(byte) => {
                result.push(modifiers.into_iter().rev().fold(byte, modify));
                return Some(());
            }
        }
    }
}

fn operand(modifier: u8, bytes: &mut std::iter::Peekable<std::str::Bytes<'_>>) -> Option<Operand> {
    if modifier != b'c' && bytes.next()? != b'-' {
        return None;
    }
    let byte = bytes.next()?;
    if byte != b'\\' {
        return Some(Operand::Byte(byte));
    }
    if matches!(bytes.peek(), Some(b'c' | b'C' | b'M')) {
        return Some(Operand::Modifier(bytes.next()?));
    }
    escaped(bytes).map(Operand::Byte)
}

fn escaped(bytes: &mut std::iter::Peekable<std::str::Bytes<'_>>) -> Option<u8> {
    let mut result = Vec::new();
    super::append(bytes, &mut result)?;
    (result.len() == 1).then(|| result[0])
}

fn modify(byte: u8, modifier: u8) -> u8 {
    match modifier {
        b'M' => byte | 0x80,
        _ if byte == b'?' => 0x7f,
        _ => byte & 0x9f,
    }
}
