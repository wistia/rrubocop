use super::*;

pub(super) fn comparison<'n>(c: &Context<'_>, n: Node<'n>) -> Option<(Node<'n>, Node<'n>)> {
    if n.kind() == "binary" && operator(c, n) == "==" {
        field(n, "left").zip(field(n, "right"))
    } else if ["==", "eql?", "equal?"].contains(&c.method(n)) {
        field(n, "receiver").zip(single_argument(c, n))
    } else {
        None
    }
}
fn single_argument<'n>(c: &Context<'_>, n: Node<'n>) -> Option<Node<'n>> {
    let args = semantic_args(n);
    (args.len() == 1 && !safe(c, n)).then(|| args[0])
}

pub(super) fn comparison_end(c: &Context<'_>, n: Node<'_>) -> usize {
    let bytes = &c.source.as_bytes()[n.start_byte()..send_end(n)];
    n.start_byte()
        + bytes
            .iter()
            .rposition(|byte| !byte.is_ascii_whitespace())
            .map_or(0, |index| index + 1)
}

pub(super) fn class_call<'n>(c: &Context<'_>, n: Node<'n>, named: bool) -> Option<Node<'n>> {
    if !plain_call(c, n) {
        return None;
    }
    let class = if named { field(n, "receiver")? } else { n };
    (c.method(class) == "class" && plain_call(c, class)).then_some(class)
}

fn plain_call(c: &Context<'_>, n: Node<'_>) -> bool {
    !safe(c, n) && block(n).is_none() && semantic_args(n).is_empty()
}
