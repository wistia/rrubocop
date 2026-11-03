use crate::cop::shared::ruby_expression::{plain_string, regular_send_name, regular_send_receiver};
use crate::cop::shared::{
    argument_nodes, call_receiver, for_each_descendant, named_kids, node_text, push_replace,
    send_end,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct FilePath;
fn constant(source: &SourceFile, node: Node<'_>, name: &str) -> bool {
    matches!(node.kind(), "constant" | "scope_resolution")
        && node_text(source, node)
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .trim_start_matches("::")
            == name
}
fn root(source: &SourceFile, node: Node<'_>) -> bool {
    regular_send_name(source, node).is_some_and(|n| n.as_ref() == b"root")
        && argument_nodes(node).is_empty()
        && regular_send_receiver(source, node)
            .flatten()
            .is_some_and(|r| constant(source, r, "Rails"))
}
fn contains_root(source: &SourceFile, node: Node<'_>) -> bool {
    let mut found = root(source, node);
    for_each_descendant(node, |n| found |= root(source, n));
    found
}
fn join(source: &SourceFile, node: Node<'_>) -> bool {
    regular_send_name(source, node).is_some_and(|n| n.as_ref() == b"join")
}
fn safe_interpolation_suffix(value: &str) -> bool {
    !value.contains(['"', '\\'])
}
fn safe_double(value: &str) -> bool {
    !value.contains(['"', '\\', '\n', '\r'])
        && !value.contains("#{")
        && !value.contains("#@")
        && !value.contains("#$")
}
impl Cop for FilePath {
    fn name(&self) -> &'static str {
        "Rails/FilePath"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "string", "chained_string", "heredoc_body"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let arguments_style = config.get_str("EnforcedStyle", "slashes") == "arguments";
        let mut edits = Vec::new();
        let mut offense = false;
        let mut to_s = false;
        if join(source, node) {
            let args = argument_nodes(node);
            if call_receiver(node).is_some_and(|r| constant(source, r, "File"))
                && args.iter().any(|a| contains_root(source, *a))
            {
                offense = true;
                to_s = true;
                if args.first().is_some_and(|a| root(source, *a))
                    && !args[1..]
                        .iter()
                        .any(|a| matches!(a.kind(), "array" | "splat_argument"))
                    && node
                        .child_by_field_name("arguments")
                        .is_some_and(|a| source.as_bytes()[a.start_byte()] == b'(')
                {
                    let receiver = call_receiver(node).unwrap();
                    edits.push((
                        receiver.start_byte(),
                        receiver.end_byte(),
                        "Rails.root".to_owned(),
                    ));
                    let first = args[0];
                    let end = args.get(1).map_or(first.end_byte(), |n| n.start_byte());
                    edits.push((first.start_byte(), end, String::new()));
                    for argument in &args[1..] {
                        if let Some(value) = plain_string(source, *argument) {
                            edits.push((
                                argument.start_byte(),
                                argument.end_byte(),
                                crate::cop::shared::ruby_literal::string_inspect(
                                    value.strip_prefix('/').unwrap_or(&value),
                                ),
                            ));
                        }
                    }
                    edits.push((send_end(node), send_end(node), ".to_s".to_owned()));
                }
            } else if call_receiver(node).is_some_and(|r| contains_root(source, r)) {
                if !arguments_style
                    && args.len() > 1
                    && args.iter().all(|a| plain_string(source, *a).is_some())
                {
                    offense = true;
                    let values: Vec<_> = args
                        .iter()
                        .filter_map(|a| plain_string(source, *a))
                        .collect();
                    let value = values.join("/");
                    if safe_double(&value) && !values.iter().skip(1).any(|s| s.starts_with('/')) {
                        edits.push((
                            args[0].start_byte(),
                            args[0].end_byte(),
                            format!("\"{value}\""),
                        ));
                        for pair in args.windows(2) {
                            edits.push((pair[0].end_byte(), pair[1].end_byte(), String::new()));
                        }
                    }
                } else if arguments_style
                    && args.iter().any(|a| {
                        plain_string(source, *a).is_some() && node_text(source, *a).contains('/')
                    })
                {
                    offense = true;
                    for arg in args {
                        if plain_string(source, arg).is_none() {
                            continue;
                        }
                        let text = node_text(source, arg);
                        let Some(index) = text.find('/') else {
                            continue;
                        };
                        let close = arg.end_byte().saturating_sub(1);
                        let slash = arg.start_byte() + index;
                        let rest = &source.as_bytes()[slash..close];
                        let rest = String::from_utf8_lossy(rest)
                            .strip_prefix('/')
                            .unwrap_or(&String::from_utf8_lossy(rest))
                            .to_owned();
                        if safe_double(&rest)
                            && !plain_string(source, arg).is_some_and(|v| v.starts_with('/'))
                        {
                            edits.push((slash, close, String::new()));
                            edits.push((arg.end_byte(), arg.end_byte(), format!(", \"{rest}\"")));
                        }
                    }
                }
            }
        } else if matches!(node.kind(), "string" | "chained_string" | "heredoc_body")
            && contains_root(source, node)
        {
            let children: Vec<_> = named_kids(node)
                .into_iter()
                .filter(|n| n.kind() != "comment")
                .collect();
            if children
                .iter()
                .skip(1)
                .any(|n| n.kind() == "string_content" && node_text(source, *n).starts_with(':'))
            {
                return;
            }
            if let Some(index) = children.iter().position(|n| contains_root(source, *n)) {
                if let Some(after) = children.get(index + 1) {
                    let text = node_text(source, *after);
                    let interpolation = children[index];
                    let embedded: Vec<_> = named_kids(interpolation)
                        .into_iter()
                        .filter(|n| n.kind() != "comment")
                        .collect();
                    let expression = embedded.first().copied();
                    if after.kind() == "string_content" && text.starts_with('/') {
                        offense = true;
                        if let Some(expression) = expression {
                            let suffix = &source.as_bytes()
                                [after.start_byte()..children.last().unwrap().end_byte()];
                            let suffix = String::from_utf8_lossy(suffix);
                            let suffix = suffix.strip_prefix('/').unwrap_or(&suffix);
                            if !safe_interpolation_suffix(suffix) {
                                edits.clear();
                            } else if root(source, expression) {
                                edits.push((
                                    expression.start_byte(),
                                    expression.end_byte(),
                                    format!("Rails.root.join(\"{suffix}\")"),
                                ));
                            } else if join(source, expression) {
                                if let Some(last) = argument_nodes(expression).last() {
                                    edits.push((
                                        last.end_byte(),
                                        last.end_byte(),
                                        format!(", \"{suffix}\""),
                                    ));
                                }
                            }
                            if !edits.is_empty() {
                                edits.push((
                                    after.start_byte(),
                                    children.last().unwrap().end_byte(),
                                    String::new(),
                                ));
                            }
                        }
                    } else if after.kind() == "string_content"
                        && text.starts_with('.')
                        && text.as_bytes().get(1).is_some_and(u8::is_ascii_alphabetic)
                    {
                        offense = true;
                        if let Some(expression) = expression.filter(|n| {
                            join(source, *n) && safe_double(&text) && !text.contains('\'')
                        }) {
                            if let Some(last) = argument_nodes(expression)
                                .last()
                                .copied()
                                .filter(|n| plain_string(source, *n).is_some())
                            {
                                edits.push((last.end_byte() - 1, last.end_byte() - 1, text));
                                edits.push((after.start_byte(), after.end_byte(), String::new()));
                            }
                        }
                    }
                }
            }
        }
        if !offense {
            return;
        }
        let mut comment = false;
        for_each_descendant(node, |n| comment |= n.kind() == "comment");
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let (_, last_col) = source.offset_to_line_col(send_end(node));
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!(
                "Prefer `Rails.root.join('{}'){}`.",
                if arguments_style {
                    "path', 'to"
                } else {
                    "path/to"
                },
                if to_s { ".to_s" } else { "" }
            ),
        );
        d.highlight_length = last_col.saturating_sub(col);
        if let Some(severity) = config.severity {
            d.severity = severity;
        }
        d.correctable = !config.autocorrect_disabled() && !comment && !edits.is_empty();
        if d.correctable {
            for (start, end, text) in edits {
                d.corrected |= push_replace(&mut corrections, start, end, text, self.name());
            }
        }
        diagnostics.push(d);
    }
}
