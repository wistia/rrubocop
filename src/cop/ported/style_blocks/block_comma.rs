use super::super::context::*;
use tree_sitter::Node;
pub(super) fn block_comma(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if n.parent().is_some_and(|p| p.kind() == "lambda") {
        return vec![];
    }
    let Some(p) = field(n, "parameters") else {
        return vec![];
    };
    if argument_count(p) < 2 {
        return vec![];
    }
    let first = first_parameters(n, p);
    if first != p && argument_count(first) > 1 {
        return vec![];
    }
    let Some(offset) = comma_offset(c, first) else {
        return vec![];
    };
    vec![
        Finding::range(
            offset,
            offset + 1,
            "Useless trailing comma present in block arguments.",
        )
        .edit_with(c, offset, offset + 1, || ""),
    ]
}

fn argument_count(n: Node<'_>) -> usize {
    descendants(n)
        .filter(|p| {
            matches!(
                p.kind(),
                "identifier" | "optional_parameter" | "keyword_parameter"
            )
        })
        .count()
}

fn comma_offset(c: &Context<'_>, p: Node<'_>) -> Option<usize> {
    let text = c.text(p).trim_end_matches('|').trim_end();
    text.ends_with(',').then(|| p.start_byte() + text.len() - 1)
}

fn first_parameters<'n>(n: Node<'n>, parameters: Node<'n>) -> Node<'n> {
    n.parent()
        .into_iter()
        .flat_map(descendants)
        .find(|child| child.kind() == parameters.kind())
        .unwrap_or(parameters)
}

#[cfg(test)]
mod original_tests {
    #[test]
    fn original_spec_states() {
        let registry = crate::cop::CopRegistry::default_registry();
        crate::testutil::assert_original_spec_parity(
            registry.get("Style/TrailingCommaInBlockArgs").unwrap(),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/trailing_comma_in_block_args/original-parity.json"
            )),
        );
    }
}
