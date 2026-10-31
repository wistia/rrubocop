use super::context::*;
use crate::cop::CopRegistry;
use tree_sitter::Node;
mod autoload;
mod eval;
mod range;
mod shadow;
pub(super) fn register(r: &mut CopRegistry) {
    super::rules!(r;
        ("Lint/MixedRegexpCaptureTypes", &["regex"], mixed, false, true, false),
        ("Style/EvalWithLocation", &["call"], eval::check, true, true, false),
        ("Cops/NoUndiscoverableAutoLoadedConstants", &["class","module","assignment","operator_assignment","constant","scope_resolution"], autoload::check, false, true, true)
    );
    r.register(Box::new(range::OutOfRangeRegexpRef));
    r.register(Box::new(shadow::ShadowedArgument));
}
fn mixed(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if super::style_regexp::capture_counts(c, n)
        .is_some_and(|(named, numbered)| named > 0 && numbered > 0)
    {
        vec![Finding::new(
            n,
            "Do not mix named captures and numbered captures in a Regexp literal.",
        )]
    } else {
        vec![]
    }
}
