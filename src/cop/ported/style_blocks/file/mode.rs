use super::*;
use std::borrow::Cow;

pub(super) fn file_mode<'a>(c: &Context<'a>, n: Node<'_>, writing: bool) -> Option<Cow<'a, str>> {
    if !file_open(c, n) {
        return None;
    }
    let a = semantic_args(n);
    if !argument_count(&a, writing) {
        return None;
    }
    let mode = mode_value(c, &a)?;
    (valid_mode(&mode, writing) && valid_options(c, &a)).then_some(mode)
}
fn mode_value<'a>(c: &Context<'a>, arguments: &[Node<'_>]) -> Option<Cow<'a, str>> {
    match arguments.get(1).filter(|a| a.kind() != "block_argument") {
        Some(argument) if argument.kind() == "heredoc_beginning" => None,
        Some(argument) => c.ruby_static_string(*argument),
        None => Some(Cow::Borrowed("r")),
    }
}
fn valid_mode(mode: &str, writing: bool) -> bool {
    if writing {
        ["w", "wt", "wb", "w+", "w+t", "w+b"].contains(&mode)
    } else {
        ["r", "rt", "rb", "r+", "r+t", "r+b"].contains(&mode)
    }
}
fn valid_options(c: &Context<'_>, arguments: &[Node<'_>]) -> bool {
    match &arguments[1..] {
        [] => true,
        [pass] if pass.kind() == "block_argument" => read_argument(c, *pass),
        [mode] => c.ruby_static_string(*mode).is_some(),
        [mode, pass] => c.ruby_static_string(*mode).is_some() && read_argument(c, *pass),
        _ => false,
    }
}
pub(super) fn read_argument(c: &Context<'_>, n: Node<'_>) -> bool {
    n.kind() == "block_argument"
        && kids(n)
            .first()
            .is_some_and(|symbol| c.static_symbol(*symbol).as_deref() == Some("read"))
}

fn file_open(c: &Context<'_>, n: Node<'_>) -> bool {
    c.method(n) == "open" && ["File", "::File"].contains(&c.receiver(n))
}

fn argument_count(arguments: &[Node<'_>], writing: bool) -> bool {
    if writing {
        arguments.len() == 2
    } else {
        !arguments.is_empty()
    }
}
