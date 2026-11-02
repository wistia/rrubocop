use super::*;

pub(super) fn comparison<'n>(c: &Context<'_>, n: Node<'n>) -> Option<(Node<'n>, Node<'n>)> {
    if n.kind() == "binary" && operator(c, n) == "==" {
        field(n, "left").zip(field(n, "right"))
    } else if ["==", "eql?", "equal?"].contains(&c.method(n)) {
        field(n, "receiver").zip(single_argument(n))
    } else {
        None
    }
}
fn single_argument(n: Node<'_>) -> Option<Node<'_>> {
    let args = semantic_args(n);
    (args.len() == 1).then(|| args[0])
}

pub(super) fn comparison_end(c: &Context<'_>, n: Node<'_>) -> usize {
    n.start_byte() + c.text(n)[..send_end(n) - n.start_byte()].trim_end().len()
}

pub(super) fn class_call<'n>(c: &Context<'_>, n: Node<'n>, named: bool) -> Option<Node<'n>> {
    if !plain_call(n) {
        return None;
    }
    let class = if named { field(n, "receiver")? } else { n };
    (c.method(class) == "class" && plain_call(class)).then_some(class)
}

fn plain_call(n: Node<'_>) -> bool {
    block(n).is_none() && semantic_args(n).is_empty()
}
