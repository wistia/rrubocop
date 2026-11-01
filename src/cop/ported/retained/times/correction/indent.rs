pub(super) fn deindent(text: &str, start: usize, end: usize) -> String {
    text.split('\n')
        .enumerate()
        .map(|(index, line)| {
            if index == 0 || end <= start || !indentation(line, start, end) {
                return line.to_owned();
            }
            format!("{}{}", &line[..start], &line[end.min(line.len())..])
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn indentation(line: &str, start: usize, end: usize) -> bool {
    line.as_bytes()
        .get(start..end.min(line.len()))
        .is_some_and(|bytes| bytes.iter().all(|byte| matches!(byte, b' ' | b'\t')))
}
