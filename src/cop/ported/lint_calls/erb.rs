use super::super::context::*;
use tree_sitter::Node;
pub(super) fn erb(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !legacy_erb(c, n) {
        return vec![];
    }
    let arguments = semantic_args(n);
    erb_findings(
        c,
        &arguments,
        &c.render_edit(|| good_arguments(c, n, &arguments)),
        correctable_arguments(&arguments),
    )
}
fn erb_findings(
    c: &Context<'_>,
    arguments: &[Node<'_>],
    good: &str,
    correctable: bool,
) -> Vec<Finding> {
    diagnostic_arguments(arguments)
        .iter()
        .skip(1)
        .take(3)
        .enumerate()
        .filter(|(_, a)| !is_options(**a))
        .map(|(i, arg)| erb_finding(c, arguments, *arg, i, good, correctable))
        .collect()
}
fn erb_finding(
    c: &Context<'_>,
    arguments: &[Node<'_>],
    arg: Node<'_>,
    index: usize,
    good: &str,
    correctable: bool,
) -> Finding {
    let finding = Finding::new(arg, message(index, c.text(arg)));
    if !correctable {
        return finding;
    }
    finding.edit_with(
        c,
        arguments[0].start_byte(),
        arguments.last().unwrap().end_byte(),
        || good,
    )
}

fn legacy_erb(c: &Context<'_>, n: Node<'_>) -> bool {
    if c.method(n) != "new"
        || !["ERB", "::ERB"].contains(&c.receiver(n))
        || c.config.get_f64("TargetRubyVersion", 2.7) < 2.6
    {
        return false;
    }
    let a = semantic_args(n);
    a.len() >= 2 && !(a.len() == 2 && is_options(a[1]))
}
fn is_options(n: Node<'_>) -> bool {
    matches!(n.kind(), "hash" | "pair" | "hash_splat_argument")
}

fn diagnostic_arguments<'n>(arguments: &[Node<'n>]) -> Vec<Node<'n>> {
    let mut keywords = false;
    arguments
        .iter()
        .copied()
        .filter(|argument| {
            let current = matches!(argument.kind(), "pair" | "hash_splat_argument");
            let include = !current || !keywords;
            keywords = current;
            include
        })
        .collect()
}

fn correctable_arguments(arguments: &[Node<'_>]) -> bool {
    !arguments.get(2).is_some_and(|arg| is_options(*arg))
        && !arguments.iter().take(4).skip(2).any(|arg| {
            matches!(
                arg.kind(),
                "splat_argument" | "hash_splat_argument" | "block_argument"
            )
        })
}
fn good_arguments(c: &Context<'_>, n: Node<'_>, a: &[Node<'_>]) -> String {
    let mut good = vec![c.text(a[0]).to_string()];
    for (i, key) in [(2, "trim_mode"), (3, "eoutvar")] {
        if let Some(value) = legacy_or_keyword(c, n, a, i, key) {
            good.push(format!("{key}: {}", c.text(value)));
        }
    }
    good.join(", ")
}
fn legacy_or_keyword<'a>(
    c: &Context<'_>,
    n: Node<'a>,
    a: &[Node<'a>],
    i: usize,
    key: &str,
) -> Option<Node<'a>> {
    a.get(i)
        .filter(|v| !is_options(**v))
        .copied()
        .or_else(|| c.pair_value(n, key))
}
fn message(index: usize, value: &str) -> String {
    match index {
        0 => "Passing safe_level with the 2nd argument of `ERB.new` is deprecated. Do not use it, and specify other arguments as keyword arguments.".into(),
        1 => format!("Passing trim_mode with the 3rd argument of `ERB.new` is deprecated. Use keyword argument like `ERB.new(str, trim_mode: {value})` instead."),
        _ => format!("Passing eoutvar with the 4th argument of `ERB.new` is deprecated. Use keyword argument like `ERB.new(str, eoutvar: {value})` instead."),
    }
}
