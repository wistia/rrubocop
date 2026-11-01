use super::*;

pub(super) fn separated(n: Node<'_>) -> bool {
    !descendants(n).any(|node| node.kind() == "comment") && args(n).into_iter().all(constant)
}

pub(super) fn grouped(c: &Context<'_>, mixins: &[Node<'_>]) -> bool {
    mixins
        .iter()
        .all(|n| !args(*n).is_empty() && separated(*n) && !suffix_comment(c, *n))
        && consecutive(c, mixins)
}

fn consecutive(c: &Context<'_>, mixins: &[Node<'_>]) -> bool {
    mixins.windows(2).all(|pair| {
        c.source.as_bytes()[pair[0].end_byte()..pair[1].start_byte()]
            .iter()
            .all(|byte| byte.is_ascii_whitespace() || *byte == b';')
    })
}

fn suffix_comment(c: &Context<'_>, n: Node<'_>) -> bool {
    c.source.as_bytes()[n.end_byte()..]
        .split(|byte| *byte == b'\n')
        .next()
        .is_some_and(|line| line.contains(&b'#'))
}

fn constant(n: Node<'_>) -> bool {
    n.kind() == "constant"
        || n.kind() == "scope_resolution"
            && field(n, "name").is_some_and(|name| name.kind() == "constant")
            && field(n, "scope").is_none_or(constant)
}
