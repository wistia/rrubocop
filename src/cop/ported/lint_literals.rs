mod ambiguous;
use ambiguous::ambiguous;
mod boolean_symbol;
use boolean_symbol::boolean_symbol;
mod deprecated;
use deprecated::deprecated;
mod empty;
use empty::empty;
mod interpolation;
use interpolation::interpolation;
mod nested_percent;
use nested_percent::nested_percent;
mod triple;
use crate::cop::CopRegistry;
use triple::triple;
pub(super) fn register(r: &mut CopRegistry) {
    super::rules!(r;
        ("Lint/AmbiguousAssignment", &["assignment"], ambiguous, false, true, false),
        ("Lint/BooleanSymbol", &["simple_symbol","delimited_symbol","hash_key_symbol"], boolean_symbol, true, false, false),
        ("Lint/DeprecatedConstants", &["constant","scope_resolution"], deprecated, true, true, false),
        ("Lint/EmptyExpression", &["parenthesized_statements"], empty, false, true, false),
        ("Lint/InterpolationCheck", &["string"], interpolation, true, false, false),
        ("Lint/NestedPercentLiteral", &["string_array","symbol_array"], nested_percent, false, true, false),
        ("Lint/TripleQuotes", &["chained_string"], triple, true, true, false)
    );
}
