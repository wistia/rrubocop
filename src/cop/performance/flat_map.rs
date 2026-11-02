//! Combine a one-level flatten with the preceding mapping operation.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, for_each_descendant, method_node, node_text,
    push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct FlatMap;
impl Cop for FlatMap {
    fn name(&self) -> &'static str {
        "Performance/FlatMap"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"flatten", b"flatten!"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(flatten @ (b"flatten" | b"flatten!")) = call_method_name(source, node) else {
            return;
        };
        let Some(map) = call_receiver(node) else {
            return;
        };
        let Some(method @ (b"map" | b"collect")) = call_method_name(source, map) else {
            return;
        };
        let map_args = argument_nodes(map);
        let block = map.child_by_field_name("block");
        if !(block.is_some() && map_args.is_empty()
            || block.is_none() && map_args.len() == 1 && map_args[0].kind() == "block_argument")
        {
            return;
        }
        let args = argument_nodes(node);
        let level_one = args.first().is_some_and(|n| numeric_one(source, *n));
        let integer_level = args.first().is_some_and(|n| {
            n.kind() == "integer"
                || n.kind() == "unary"
                    && n.child_by_field_name("operand")
                        .is_some_and(|n| n.kind() == "integer")
        });
        let missing_level = args.is_empty()
            || args.first().is_some_and(|n| {
                n.kind() == "nil" || n.kind() == "call" && call_receiver(*n).is_none()
            });
        let warn = missing_level && config.get_bool("EnabledForFlattenWithoutParams", false);
        if !level_one && !warn {
            return;
        }
        let Some(selector) = method_node(map) else {
            return;
        };
        let mut message = format!(
            "Use `flat_map` instead of `{}...{}`.",
            String::from_utf8_lossy(method),
            String::from_utf8_lossy(flatten)
        );
        if warn {
            message.push_str(" Beware, `flat_map` only flattens 1 level and `flatten` can be used to flatten multiple levels.");
        }
        let (line, col) = source.offset_to_line_col(selector.start_byte());
        let mut d = self.diagnostic(source, line, col, message);
        let bytes = &source.as_bytes()[selector.start_byte()..node.end_byte()];
        let end = bytes
            .iter()
            .position(|b| *b == b'\n')
            .unwrap_or(bytes.len());
        d.highlight_length = String::from_utf8_lossy(&bytes[..end]).chars().count();
        let mut breaks = false;
        if let Some(block) = block {
            for_each_descendant(block, |n| breaks |= n.kind() == "break");
        }
        let guarded = !integer_level
            || flatten == b"flatten!"
            || args.len() != 1
            || node.child_by_field_name("block").is_some()
            || source.as_bytes()[map.end_byte()..node.end_byte()].contains(&b'#')
            || breaks;
        d.correctable = level_one && !guarded && !config.autocorrect_disabled();
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                selector.start_byte(),
                selector.end_byte(),
                "flat_map",
                self.name(),
            );
            push_replace(
                &mut corrections,
                map.end_byte(),
                node.end_byte(),
                "",
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
fn numeric_one(source: &SourceFile, node: Node<'_>) -> bool {
    let text = node_text(source, node).replace('_', "");
    let text = text.trim_start_matches('+');
    for (prefix, radix) in [
        ("0x", 16),
        ("0X", 16),
        ("0o", 8),
        ("0O", 8),
        ("0b", 2),
        ("0B", 2),
    ] {
        if let Some(value) = text.strip_prefix(prefix) {
            return u64::from_str_radix(value, radix).ok() == Some(1);
        }
    }
    text.parse::<f64>().ok() == Some(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FlatMap, "cops/performance/flat_map");

    #[test]
    fn preserves_bang_return_values_and_break_values() {
        for before in [
            b"items.map { |item| item }.flatten!(1)\n".as_slice(),
            b"items.map { |item| break [[7]] }.flatten(1)\n",
        ] {
            let source = SourceFile::from_bytes("test.rb", before.to_vec());
            let tree = crate::parse::parse_ruby(&source).unwrap();
            let config = CopConfig::default();
            let mut diagnostics = vec![];
            let mut edits = vec![];
            crate::cop::walker::BatchedWalker::new(vec![&FlatMap], vec![&config]).walk(
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
}
