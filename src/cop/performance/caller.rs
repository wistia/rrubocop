//! Limit stack collection when selecting a single caller frame.
use crate::cop::shared::ruby_expression::integer;
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, collect_comments, named_kids, node_bytes,
    node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use num_bigint::{BigInt, Sign};
use tree_sitter::Node;
pub struct Caller;
impl Cop for Caller {
    fn name(&self) -> &'static str {
        "Performance/Caller"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "element_reference"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"first", b"[]"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let (receiver, index) = if node.kind() == "element_reference" {
            let Some(object) = node.child_by_field_name("object") else {
                return;
            };
            let args: Vec<_> = named_kids(node)
                .into_iter()
                .filter(|n| *n != object && n.kind() != "comment")
                .collect();
            if args.len() != 1 {
                return;
            }
            let Some(index) = integer(source, args[0]) else {
                return;
            };
            (object, index)
        } else {
            if node
                .child_by_field_name("operator")
                .is_some_and(|op| node_bytes(source, op) == b"&.")
            {
                return;
            }
            let Some(receiver) = call_receiver(node) else {
                return;
            };
            let args = argument_nodes(node);
            match call_method_name(source, node) {
                Some(b"first") if args.is_empty() => (receiver, BigInt::from(0)),
                Some(b"[]") if args.len() == 1 => {
                    let Some(index) = integer(source, args[0]) else {
                        return;
                    };
                    (receiver, index)
                }
                _ => return,
            }
        };
        let Some((method, offset)) = caller(source, receiver) else {
            return;
        };
        let position = &offset + &index;
        let preferred = format!("{method}({position}..{position}).first");
        let end = super::send_range::end(node);
        let current = String::from_utf8_lossy(&source.as_bytes()[node.start_byte()..end]);
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!("Use `{preferred}` instead of `{current}`."),
        );
        d.highlight_length = current.lines().next().unwrap_or("").chars().count();
        d.correctable = index.sign() != Sign::Minus
            && offset.sign() != Sign::Minus
            && !config.autocorrect_disabled()
            && !collect_comments(node).iter().any(|n| n.start_byte() < end);
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                node.start_byte(),
                end,
                preferred,
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
fn caller(source: &SourceFile, node: Node<'_>) -> Option<(String, BigInt)> {
    let name = if node.kind() == "identifier" && !crate::cop::ported::local_identifier(source, node)
    {
        node_text(source, node)
    } else {
        if call_receiver(node).is_some() || node.child_by_field_name("block").is_some() {
            return None;
        }
        String::from_utf8(call_method_name(source, node)?.to_vec()).ok()?
    };
    if !matches!(name.as_str(), "caller" | "caller_locations") {
        return None;
    }
    let args = argument_nodes(node);
    let offset = match args.as_slice() {
        [] => BigInt::from(1),
        [argument] => integer(source, *argument)?,
        _ => return None,
    };
    Some((name, offset))
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Caller, "cops/performance/caller");

    #[test]
    fn preserves_negative_index_semantics() {
        let before = b"caller[-1]\n";
        let source = SourceFile::from_bytes("test.rb", before.to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let config = CopConfig::default();
        let mut diagnostics = vec![];
        let mut edits = vec![];
        crate::cop::walker::BatchedWalker::new(vec![&Caller], vec![&config]).walk(
            &source,
            tree.root_node(),
            &mut diagnostics,
            Some(&mut edits),
        );
        assert!(edits.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].correctable);
    }
}
