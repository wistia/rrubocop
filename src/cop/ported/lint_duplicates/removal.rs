use super::*;

pub(super) fn removes_closing_delimiter(n: Node<'_>, start: usize, end: usize) -> bool {
    ancestors(n).any(|parent| {
        parent.start_byte() < start
            && parent.end_byte() <= end
            && parent
                .child(parent.child_count().saturating_sub(1) as u32)
                .is_some_and(|last| matches!(last.kind(), "end" | ")" | "]" | "}"))
    })
}
pub(super) fn whole_lines(c: &Context<'_>, n: Node<'_>) -> (usize, usize) {
    (
        c.source
            .line_start(n.start_position().row + 1)
            .unwrap_or(n.start_byte()),
        c.source
            .line_start(n.end_position().row + 2)
            .unwrap_or(c.source.as_bytes().len()),
    )
}
