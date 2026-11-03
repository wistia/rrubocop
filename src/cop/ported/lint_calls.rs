mod raise_exception;
use raise_exception::raise_exception;
mod uri;
use uri::{uri_escape, uri_regexp};
mod identity;
use identity::identity;
mod redundant_with;
use redundant_with::{with_index, with_object};
mod erb;
use erb::erb;
use crate::cop::CopRegistry;
use crate::cop::lint::trailing_comma_in_attribute_declaration::TrailingCommaInAttributeDeclaration;
pub(super) fn register(r: &mut CopRegistry) {
    super::rules!(r;
        ("Lint/RaiseException", &["call"], raise_exception, true, true, false),
        ("Lint/UriRegexp", &["call"], uri_regexp, true, true, false),
        ("Lint/UriEscapeUnescape", &["call"], uri_escape, false, true, false),
        ("Lint/IdentityComparison", &["binary","call"], identity, true, true, false),
        ("Lint/RedundantWithIndex", &["call"], with_index, true, true, false),
        ("Lint/RedundantWithObject", &["call"], with_object, true, true, false),
        ("Lint/ErbNewArguments", &["call"], erb, true, true, false),
    );
    r.register(Box::new(TrailingCommaInAttributeDeclaration));
}

#[cfg(test)]
mod comment_tests;
