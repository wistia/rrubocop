use super::super::context::*;
use num_bigint::BigInt;
use tree_sitter::Node;
pub(super) fn random(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some((rand, offset, reverse)) = random_parts(c, n) else {
        return vec![];
    };
    if !rand_candidate(c, rand) {
        return vec![];
    }
    let a = semantic_args(rand);
    if a.len() != 1 {
        return vec![];
    }
    let Some(bounds) = boundaries(c, a[0]) else {
        return vec![];
    };
    vec![random_finding(
        c,
        n,
        rand,
        shifted(c, n, bounds, offset, reverse),
    )]
}
fn rand_candidate(c: &Context<'_>, n: Node<'_>) -> bool {
    c.method(n) == "rand"
        && ["", "Random", "::Random", "Kernel", "::Kernel"].contains(&c.receiver(n))
}
fn shifted(
    c: &Context<'_>,
    n: Node<'_>,
    (lo, hi): (BigInt, BigInt),
    offset: BigInt,
    reverse: bool,
) -> (BigInt, BigInt) {
    let op = if n.kind() == "binary" {
        operator(c, n)
    } else {
        c.method(n)
    };
    if op == "-" && reverse {
        (&offset - hi, &offset - lo)
    } else if ["-", "pred"].contains(&op) {
        (lo - &offset, hi - &offset)
    } else {
        (lo + &offset, hi + &offset)
    }
}
fn random_finding(
    c: &Context<'_>,
    n: Node<'_>,
    rand: Node<'_>,
    (lo, hi): (BigInt, BigInt),
) -> Finding {
    Finding::new(
        n,
        "Prefer ranges when generating random numbers instead of integers with offsets.",
    )
    .replace_with(c, n, || {
        let prefix = if c.receiver(rand).is_empty() {
            "rand".into()
        } else {
            format!("{}.rand", c.receiver(rand))
        };
        format!("{prefix}({lo}..{hi})")
    })
}
fn random_parts<'a>(c: &Context<'_>, n: Node<'a>) -> Option<(Node<'a>, BigInt, bool)> {
    if n.kind() == "binary" && ["+", "-"].contains(&operator(c, n)) {
        return binary_parts(c, n);
    }
    if ["succ", "pred", "next"].contains(&c.method(n)) && semantic_args(n).is_empty() {
        return Some((field(n, "receiver")?, BigInt::from(1u8), false));
    }
    None
}
fn binary_parts<'a>(c: &Context<'_>, n: Node<'a>) -> Option<(Node<'a>, BigInt, bool)> {
    let l = field(n, "left")?;
    let r = field(n, "right")?;
    if let Some(i) = integer(c, r) {
        return Some((l, i, false));
    }
    if let Some(i) = integer(c, l) {
        return Some((r, i, true));
    }
    None
}
fn integer(c: &Context<'_>, n: Node<'_>) -> Option<BigInt> {
    let text = c.text(n).replace('_', "");
    let (radix, digits) = digits(text.trim_start_matches(['-', '+']));
    let number = BigInt::parse_bytes(digits.as_bytes(), radix)?;
    Some(if text.starts_with('-') {
        -number
    } else {
        number
    })
}
fn digits(text: &str) -> (u32, &str) {
    match text.get(..2).unwrap_or("") {
        "0x" | "0X" => (16, &text[2..]),
        "0o" | "0O" => (8, &text[2..]),
        "0b" | "0B" => (2, &text[2..]),
        "0d" | "0D" => (10, &text[2..]),
        _ if text.len() > 1 && text.starts_with('0') => (8, text),
        _ => (10, text),
    }
}

fn boundaries(c: &Context<'_>, n: Node<'_>) -> Option<(BigInt, BigInt)> {
    if n.kind() == "range" {
        return range_bounds(c, n);
    }
    Some((BigInt::from(0u8), integer(c, n)? - 1u8))
}
fn range_bounds(c: &Context<'_>, n: Node<'_>) -> Option<(BigInt, BigInt)> {
    Some((
        integer(c, field(n, "begin")?)?,
        integer(c, field(n, "end")?)? - u8::from(operator(c, n) == "..."),
    ))
}
