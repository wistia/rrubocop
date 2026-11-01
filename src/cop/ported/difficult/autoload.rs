use super::super::context::*;
use tree_sitter::Node;
mod inflections;
mod location;
mod names;
const MESSAGE: &str =
    "This constant is undiscoverable by the auto loader due to where it is nested.";
pub(super) fn check(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if definition(n) {
        inspect(c, n).into_iter().collect()
    } else {
        vec![]
    }
}
fn inspect(c: &Context<'_>, n: Node<'_>) -> Option<Finding> {
    let info = c.source.cached(0, || AutoloadInfo::new(c.source));
    let (expected, acronyms) = info.0.as_ref()?;
    let actual = names::defined(c, n, acronyms, expected.len().saturating_add(10))?;
    (!compatible(expected, actual.strip_prefix("Object::").unwrap_or(&actual)))
        .then(|| Finding::new(n, MESSAGE))
}
struct AutoloadInfo(Option<(String, std::sync::Arc<inflections::Acronyms>)>);
impl AutoloadInfo {
    fn new(source: &crate::parse::source::SourceFile) -> Self {
        Self(location::resolve(source).map(|(root, relative)| {
            let acronyms = inflections::acronyms(source, &root);
            (expected_name(&relative, &acronyms), acronyms)
        }))
    }
}
fn compatible(expected: &str, actual: &str) -> bool {
    let mut expected = expected.split("::");
    let mut actual = actual.split("::");
    loop {
        match (expected.next(), actual.next()) {
            (Some(a), Some(b)) if a != b => return false,
            (Some(_), Some(_)) => {}
            _ => return true,
        }
    }
}

fn expected_name(relative: &str, acronyms: &inflections::Acronyms) -> String {
    inflections::camelize(&relative.replace(".rb", ""), acronyms)
}
fn definition(n: Node<'_>) -> bool {
    match n.kind() {
        "class" | "module" | "constant" | "scope_resolution" => true,
        "assignment" | "operator_assignment" => field(n, "left").is_some_and(constant),
        _ => false,
    }
}
fn constant(n: Node<'_>) -> bool {
    matches!(n.kind(), "constant" | "scope_resolution")
}

#[cfg(test)]
mod tests;
