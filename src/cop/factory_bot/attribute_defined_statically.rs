//! Delay factory attributes by declaring their values inside a block.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, method_node, named_kids, node_bytes,
    node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct AttributeDefinedStatically;
impl Cop for AttributeDefinedStatically {
    fn name(&self) -> &'static str {
        "FactoryBot/AttributeDefinedStatically"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[
            b"factory",
            b"ignore",
            b"trait",
            b"traits_for_enum",
            b"transient",
        ]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !matches!(
            call_method_name(source, node),
            Some(b"factory" | b"ignore" | b"trait" | b"traits_for_enum" | b"transient")
        ) {
            return;
        }
        let Some(block) = node.child_by_field_name("block") else {
            return;
        };
        let Some(body) = block.child_by_field_name("body") else {
            return;
        };
        let parameter = block
            .child_by_field_name("parameters")
            .and_then(|n| named_kids(n).into_iter().find(|n| n.kind() == "identifier"));
        for attribute in named_kids(body) {
            if attribute.kind() != "call" || attribute.child_by_field_name("block").is_some() {
                continue;
            }
            let Some(method) = call_method_name(source, attribute) else {
                continue;
            };
            if reserved(method) {
                continue;
            }
            let receiver = call_receiver(attribute);
            if receiver.is_some_and(|r| {
                r.kind() != "self"
                    && !parameter.is_some_and(|p| {
                        r.kind() == "identifier" && node_bytes(source, r) == node_bytes(source, p)
                    })
            }) {
                continue;
            }
            let values = argument_nodes(attribute);
            if values.is_empty()
                || values.iter().all(|n| n.kind() == "block_argument")
                || association(source, &values)
            {
                continue;
            }
            let all_pairs = values.iter().all(|n| n.kind() == "pair");
            let correctable = (values.len() == 1 || all_pairs) && !config.autocorrect_disabled();
            let (line, col) = source.offset_to_line_col(attribute.start_byte());
            let mut d = self.diagnostic(
                source,
                line,
                col,
                "Use a block to declare attribute values.".into(),
            );
            let bytes = node_bytes(source, attribute);
            let end = bytes
                .iter()
                .position(|b| *b == b'\n')
                .unwrap_or(bytes.len());
            d.highlight_length = String::from_utf8_lossy(&bytes[..end]).chars().count();
            d.correctable = correctable;
            if correctable {
                let Some(args) = attribute.child_by_field_name("arguments") else {
                    continue;
                };
                let bare_hash = all_pairs;
                let (left, right) = if bare_hash {
                    ("{ { ", " } }")
                } else {
                    ("{ ", " }")
                };
                if source.as_bytes().get(args.start_byte()) == Some(&b'(')
                    && method_node(attribute).is_some_and(|m| m.end_byte() == args.start_byte())
                {
                    d.corrected = push_replace(
                        &mut corrections,
                        args.start_byte(),
                        args.start_byte() + 1,
                        format!(" {left}"),
                        self.name(),
                    );
                    push_replace(
                        &mut corrections,
                        args.end_byte() - 1,
                        args.end_byte(),
                        right,
                        self.name(),
                    );
                } else {
                    d.corrected = push_replace(
                        &mut corrections,
                        values[0].start_byte(),
                        values[0].start_byte(),
                        left,
                        self.name(),
                    );
                    let last = values.last().unwrap();
                    push_replace(
                        &mut corrections,
                        last.end_byte(),
                        last.end_byte(),
                        right,
                        self.name(),
                    );
                }
            }
            diagnostics.push(d);
        }
    }
}
fn association(source: &SourceFile, values: &[Node<'_>]) -> bool {
    let first = values[0];
    let pairs = if first.kind() == "hash" {
        named_kids(first)
    } else if first.kind() == "pair" {
        values.to_vec()
    } else {
        return false;
    };
    pairs.iter().filter(|n| n.kind() == "pair").any(|pair| {
        pair.child_by_field_name("key").is_some_and(|key| {
            matches!(
                key.kind(),
                "hash_key_symbol" | "simple_symbol" | "delimited_symbol"
            ) && node_text(source, key)
                .trim_start_matches(':')
                .trim_matches(['\'', '"'])
                == "factory"
        })
    })
}
fn reserved(method: &[u8]) -> bool {
    matches!(
        method,
        b"add_attribute"
            | b"after"
            | b"association"
            | b"before"
            | b"callback"
            | b"ignore"
            | b"initialize_with"
            | b"sequence"
            | b"skip_create"
            | b"to_create"
            | b"__send__"
            | b"__id__"
            | b"nil?"
            | b"send"
            | b"object_id"
            | b"extend"
            | b"instance_eval"
            | b"initialize"
            | b"block_given?"
            | b"raise"
            | b"caller"
            | b"method"
            | b"factory"
            | b"trait"
            | b"traits_for_enum"
            | b"transient"
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        AttributeDefinedStatically,
        "cops/factory_bot/attribute_defined_statically"
    );

    #[test]
    fn retains_raw_literal_bytes_when_wrapping_the_value() {
        let before = b"factory :user do\n name \"\xff\"\nend\n";
        let source = SourceFile::from_bytes("spec/factories/users.rb", before.to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let config = CopConfig::default();
        let mut diagnostics = vec![];
        let mut edits = vec![];
        crate::cop::walker::BatchedWalker::new(vec![&AttributeDefinedStatically], vec![&config])
            .walk(
                &source,
                tree.root_node(),
                &mut diagnostics,
                Some(&mut edits),
            );
        assert_eq!(
            crate::correction::CorrectionSet::from_vec(edits).apply(before),
            b"factory :user do\n name { \"\xff\" }\nend\n"
        );
    }
}
