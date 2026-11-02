use crate::cop::shared::ruby_arguments::arguments;
use crate::cop::shared::ruby_expression::plain_string;
use crate::cop::shared::ruby_semantics::value_used;
use crate::cop::shared::{
    call_method_name, call_receiver, collect_comments, node_bytes, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn literal(source: &SourceFile, node: Node<'_>) -> bool {
    node.kind() == "regex" || plain_string(source, node).is_some()
}
fn truthiness(node: Node<'_>) -> bool {
    node.parent().is_some_and(|parent| match parent.kind() {
        "if" | "unless" | "if_modifier" | "unless_modifier" | "while" | "until"
        | "while_modifier" | "until_modifier" => {
            parent.child_by_field_name("condition") == Some(node)
        }
        "case" => parent.child_by_field_name("value") == Some(node),
        _ => false,
    })
}
fn parentheses(source: &SourceFile, node: Node<'_>) -> bool {
    match node.kind() {
        "conditional" | "range" => true,
        "binary" => node.child_by_field_name("operator").is_none_or(|op| {
            !matches!(
                node_bytes(source, op),
                b"|" | b"^"
                    | b"&"
                    | b"+"
                    | b"-"
                    | b"*"
                    | b"/"
                    | b"%"
                    | b"**"
                    | b">"
                    | b">="
                    | b"<"
                    | b"<="
                    | b"<<"
                    | b">>"
            )
        }),
        "yield" => {
            let text = node_text(source, node);
            !text
                .trim_start_matches("yield")
                .trim_start()
                .starts_with('(')
                && crate::cop::shared::named_kids(node)
                    .iter()
                    .any(|n| n.kind() != "comment")
        }
        "call" | "super" => node.child_by_field_name("arguments").is_some_and(|args| {
            args.named_child_count() > 0 && !node_bytes(source, args).starts_with(b"(")
        }),
        _ => false,
    }
}
pub struct RedundantMatch;
impl Cop for RedundantMatch {
    fn name(&self) -> &'static str {
        "Performance/RedundantMatch"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"match"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if call_method_name(source, node) != Some(b"match")
            || node.child_by_field_name("block").is_some()
            || node
                .child_by_field_name("operator")
                .is_some_and(|op| node_bytes(source, op) == b"&.")
        {
            return;
        }
        let Some(receiver) = call_receiver(node) else {
            return;
        };
        let args = arguments(node);
        if args.len() != 1 {
            return;
        }
        let Some(arg) = args[0].node() else {
            return;
        };
        if !literal(source, receiver) && !literal(source, arg)
            || value_used(source, node) && !truthiness(node)
        {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            "Use `=~` in places where the `MatchData` returned by `#match` will not be used."
                .into(),
        );
        let end = super::send_range::end(node);
        d.highlight_length = String::from_utf8_lossy(&source.as_bytes()[node.start_byte()..end])
            .split('\n')
            .next()
            .unwrap_or("")
            .chars()
            .count();
        d.correctable =
            !config.autocorrect_disabled() && (receiver.kind() == "regex" || arg.kind() == "regex");
        if d.correctable
            && literal(source, receiver)
            && !node.parent().is_some_and(|parent| parent.kind() == "case")
            && collect_comments(node).is_empty()
        {
            let value = node_text(source, arg);
            let value = if parentheses(source, arg) {
                format!("({value})")
            } else {
                value
            };
            let replacement = format!("{} =~ {value}", node_text(source, receiver));
            let mut candidate = source.as_bytes().to_vec();
            candidate.splice(node.start_byte()..end, replacement.bytes());
            let candidate = SourceFile::from_bytes("candidate.rb", candidate);
            if crate::parse::parse_ruby(&candidate).is_ok_and(|tree| !tree.root_node().has_error())
            {
                d.corrected = push_replace(
                    &mut corrections,
                    node.start_byte(),
                    end,
                    replacement,
                    self.name(),
                );
            }
        }
        diagnostics.push(d);
    }
}
