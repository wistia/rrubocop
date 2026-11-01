//! Comment / string / heredoc region map for source cops.

use tree_sitter::Node;

/// Byte ranges that are non-code (comments, strings, heredocs, regex).
#[derive(Debug, Default)]
pub struct CodeMap {
    ranges: Vec<(usize, usize)>,
}

impl CodeMap {
    pub fn from_tree(root: Node<'_>, src: &[u8]) -> Self {
        let mut ranges = Vec::new();
        collect_ranges(root, src, &mut ranges);
        Self {
            ranges: union(ranges),
        }
    }

    pub fn covers(&self, offset: usize) -> bool {
        let next = self.ranges.partition_point(|&(start, _)| start <= offset);
        next > 0 && offset < self.ranges[next - 1].1
    }
}

fn union(mut ranges: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    ranges.retain(|&(start, end)| start < end);
    ranges.sort_unstable();
    let mut count = 0;
    for index in 0..ranges.len() {
        let (start, end) = ranges[index];
        if count > 0 && start <= ranges[count - 1].1 {
            ranges[count - 1].1 = ranges[count - 1].1.max(end);
        } else {
            ranges[count] = (start, end);
            count += 1;
        }
    }
    ranges.truncate(count);
    ranges
}

fn collect_ranges(node: Node<'_>, src: &[u8], out: &mut Vec<(usize, usize)>) {
    let kind = node.kind();
    if matches!(
        kind,
        "comment"
            | "string"
            | "string_content"
            | "heredoc_body"
            | "heredoc_beginning"
            | "heredoc_end"
            | "regex"
            | "character"
            | "delimited_symbol"
            | "symbol_array"
            | "subshell"
            | "simple_symbol"
            | "hash_key_symbol"
            | "%w"
            | "%W"
            | "%i"
            | "%I"
            | "%q"
            | "%Q"
            | "%r"
            | "%s"
            | "%x"
    ) || kind.contains("string")
        || kind.contains("heredoc")
    {
        let mut start = node.start_byte();
        if permits_interpolation(node, src) {
            collect_interpolations(node, src, out, &mut start);
        }
        out.push((start, node.end_byte()));
        return;
    }
    // Still walk children for nested comments outside strings
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_ranges(child, src, out);
    }
}

fn permits_interpolation(node: Node<'_>, src: &[u8]) -> bool {
    if !matches!(
        node.kind(),
        "string" | "delimited_symbol" | "string_array" | "symbol_array"
    ) {
        return true;
    }
    let bytes = &src[node.byte_range()];
    !(bytes.starts_with(b"'")
        || bytes.starts_with(b":'")
        || matches!(bytes.get(..2), Some(b"%w" | b"%i" | b"%q" | b"%s")))
}

fn collect_interpolations(
    node: Node<'_>,
    src: &[u8],
    out: &mut Vec<(usize, usize)>,
    start: &mut usize,
) {
    if node.kind() == "interpolation" {
        out.push((*start, node.start_byte()));
        collect_interpolation(node, src, out);
        *start = node.end_byte();
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_interpolations(child, src, out, start);
    }
}

fn collect_interpolation(node: Node<'_>, src: &[u8], out: &mut Vec<(usize, usize)>) {
    let start = node.start_byte();
    if src[node.byte_range()].starts_with(b"#{") {
        out.push((start, start + 2));
        out.push((node.end_byte() - 1, node.end_byte()));
    } else {
        out.push((start, start + 1));
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_ranges(child, src, out);
    }
}

#[cfg(test)]
mod tests;
