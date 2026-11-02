pub(super) fn strip(content: &str) -> String {
    let width = content
        .lines()
        .filter(|line| !line.trim_matches([' ', '\t']).is_empty())
        .map(indentation)
        .min()
        .unwrap_or(0);
    content
        .split_inclusive('\n')
        .map(|line| unindent(line, width))
        .collect()
}

fn indentation(line: &str) -> usize {
    line.bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .fold(0, advance)
}

fn advance(column: usize, byte: u8) -> usize {
    if byte == b'\t' {
        (column / 8 + 1) * 8
    } else {
        column + 1
    }
}

fn unindent(line: &str, width: usize) -> &str {
    let mut column = 0;
    let mut offset = 0;
    for byte in line.bytes().take_while(|byte| matches!(byte, b' ' | b'\t')) {
        let next = advance(column, byte);
        if next > width {
            break;
        }
        column = next;
        offset += 1;
    }
    &line[offset..]
}
