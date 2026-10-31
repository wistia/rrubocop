use super::context::*;
use crate::cop::CopRegistry;
use tree_sitter::Node;
pub(super) fn register(r: &mut CopRegistry) {
    super::rules!(r;
        ("Style/ClassEqualityComparison", &["call","binary"], class_equality, true, false, false),
        ("Style/MixinUsage", &["call"], mixin, false, true, false),
        ("Style/OrAssignment", &["assignment","if","unless","if_modifier","unless_modifier","conditional"], or_assignment, true, true, false),
        ("Style/RedundantAssignment", &["method","singleton_method"], redundant_assignment, true, true, false),
        ("Style/RedundantConditional", &["if","unless","conditional","elsif"], redundant_conditional, true, true, false),
        ("Style/TopLevelMethodDefinition", &["method","singleton_method","call"], top_method, false, true, false),
        ("Style/UnlessLogicalOperators", &["unless","unless_modifier"], unless_logic, false, true, false),
        ("Security/CompoundHash", &["binary","operator_assignment","call"], compound_hash, false, true, false)
    );
}
fn branches(n: Node<'_>) -> (Option<Node<'_>>, Option<Node<'_>>) {
    let first = field(n, "consequence")
        .or_else(|| field(n, "body"))
        .and_then(|b| statements(b).first().copied());
    let second = field(n, "alternative").and_then(|b| statements(b).first().copied());
    if n.kind().starts_with("unless") {
        (second, first)
    } else {
        (first, second)
    }
}

mod class_equality;
use class_equality::class_equality;

mod mixin;
use mixin::mixin;

mod or_assignment;
use or_assignment::or_assignment;

mod redundant_assignment;
use redundant_assignment::redundant_assignment;

mod redundant_conditional;
use redundant_conditional::redundant_conditional;

mod top_method;
use top_method::top_method;

mod unless_logic;
use unless_logic::unless_logic;

mod compound_hash;
use compound_hash::compound_hash;
