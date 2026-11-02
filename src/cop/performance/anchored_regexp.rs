use crate::cop::CopConfig;
use crate::cop::shared::ruby_expression::static_regex_value_for_ruby;
use crate::cop::shared::{decode_ruby_double_quoted_content, is_literal_regexp_text};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(super) struct AnchoredRegexp {
    pub text: Option<String>,
    pub absolute: bool,
}

pub(super) fn read(
    source: &SourceFile,
    node: Node<'_>,
    suffix: bool,
    config: &CopConfig,
) -> Option<AnchoredRegexp> {
    if node.kind() != "regex" {
        return None;
    }
    let pattern =
        static_regex_value_for_ruby(source, node, config.get_f64("TargetRubyVersion", 3.4))?;
    let absolute = if suffix {
        pattern.strip_suffix("\\z")
    } else {
        pattern.strip_prefix("\\A")
    };
    let literal = absolute.or_else(|| {
        if config.get_bool("SafeMultiline", true) {
            None
        } else if suffix {
            pattern.strip_suffix('$')
        } else {
            pattern.strip_prefix('^')
        }
    })?;
    if !is_literal_regexp_text(literal) {
        return None;
    }
    let mut chars = literal.chars();
    let mut unsupported = false;
    while let Some(c) = chars.next() {
        if c == '\\'
            && chars
                .next()
                .is_some_and(|c| matches!(c, 'c' | 'C' | 'M' | '\n'))
        {
            unsupported = true;
            break;
        }
    }
    let text = if unsupported {
        None
    } else {
        decode_ruby_double_quoted_content(literal)
    };
    Some(AnchoredRegexp {
        text,
        absolute: absolute.is_some(),
    })
}
