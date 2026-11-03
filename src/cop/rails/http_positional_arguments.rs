use crate::cop::shared::ruby_arguments::{Argument, arguments};
use crate::cop::shared::ruby_expression::{plain_symbol, regular_send_name, regular_send_receiver};
use crate::cop::shared::{for_each_descendant, node_text, push_replace, send_end};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct HttpPositionalArguments;
struct RackTestMethods(bool);
fn name(source: &SourceFile, node: Node<'_>, names: &[&[u8]]) -> bool {
    regular_send_name(source, node).is_some_and(|name| names.contains(&name.as_ref()))
}
fn convert(source: &SourceFile, arg: &Argument<'_>, kind: &str) -> String {
    let value = if arg.hash {
        let pairs: Vec<_> = arg
            .parts()
            .into_iter()
            .filter(|n| n.kind() == "pair")
            .collect();
        if arg.parts().is_empty() {
            return String::new();
        }
        format!(
            "{{ {} }}",
            pairs
                .into_iter()
                .map(|n| node_text(source, n))
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        String::from_utf8_lossy(&source.as_bytes()[arg.start()..arg.end()]).into_owned()
    };
    format!(", {kind}: {value}")
}
impl Cop for HttpPositionalArguments {
    fn name(&self) -> &'static str {
        "Rails/HttpPositionalArguments"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn default_include(&self) -> &'static [&'static str] {
        &["**/spec/**/*", "**/test/**/*"]
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"get", b"post", b"put", b"patch", b"delete", b"head"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if config.get_f64("TargetRailsVersion", 8.1) < 5.0
            || !config.get_bool("__RailtiesInLockfile", true)
            || config.get_f64(
                "__RailtiesVersion",
                config.get_f64("TargetRailsVersion", 8.1),
            ) < 5.0
        {
            return;
        }
        if regular_send_receiver(source, node) != Some(None)
            || !name(
                source,
                node,
                &[b"get", b"post", b"put", b"patch", b"delete", b"head"],
            )
        {
            return;
        }
        let args = arguments(node);
        if args.len() < 2 {
            return;
        }
        let mut ancestor = node.parent();
        while let Some(parent) = ancestor {
            if matches!(parent.kind(), "block" | "do_block")
                && !crate::cop::shared::ruby_expression::numbered_bindings(source, parent).is_some()
                && parent
                    .parent()
                    .is_some_and(|call| name(source, call, &[b"draw", b"routes"]))
            {
                return;
            }
            ancestor = parent.parent();
        }
        let rack = source.cached(0, || {
            let mut root = node;
            while let Some(parent) = root.parent() {
                root = parent;
            }
            let mut rack = false;
            for_each_descendant(root, |n| {
                if rack
                    || regular_send_receiver(source, n) != Some(None)
                    || !name(source, n, &[b"include"])
                    || n == root
                {
                    return;
                }
                let a = arguments(n);
                if a.len() == 1
                    && a[0].node().is_some_and(|n| {
                        n.kind() == "scope_resolution"
                            && node_text(source, n)
                                .chars()
                                .filter(|c| !c.is_whitespace())
                                .collect::<String>()
                                .trim_start_matches("::")
                                == "Rack::Test::Methods"
                    })
                {
                    rack = true;
                }
            });
            RackTestMethods(rack)
        });
        if rack.0 {
            return;
        }
        let data = &args[1];
        if data.hash {
            let parts = data.parts();
            if parts.len() == 1 && parts[0].kind() == "hash_splat_argument" {
                return;
            }
            let pairs: Vec<_> = parts.into_iter().filter(|n| n.kind() == "pair").collect();
            if pairs.iter().any(|pair| {
                pair.child_by_field_name("key")
                    .and_then(|n| plain_symbol(source, n))
                    .is_some_and(|key| {
                        matches!(
                            key.as_str(),
                            "method"
                                | "params"
                                | "session"
                                | "body"
                                | "flash"
                                | "xhr"
                                | "as"
                                | "headers"
                                | "env"
                                | "to"
                        ) || key == "format" && pairs.len() == 1
                    })
            }) {
                return;
            }
        }
        let verb = regular_send_name(source, node).unwrap();
        let (line, col) = source.offset_to_line_col(data.start());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!(
                "Use keyword arguments instead of positional arguments for http call: `{}`.",
                String::from_utf8_lossy(&verb)
            ),
        );
        d.highlight_length =
            super::blank::highlight_length(source, data.start(), args.last().unwrap().end());
        if let Some(severity) = config.severity {
            d.severity = severity;
        }
        let mut comment = false;
        for_each_descendant(node, |n| comment |= n.kind() == "comment");
        d.correctable = !config.autocorrect_disabled()
            && args.len() <= 3
            && !comment
            && !args[1..]
                .iter()
                .any(|a| a.hash && a.parts().iter().any(|n| n.kind() == "hash_splat_argument"));
        if d.correctable {
            let action =
                String::from_utf8_lossy(&source.as_bytes()[args[0].start()..args[0].end()]);
            let params = convert(source, data, "params");
            let session = if args.len() > 2 {
                convert(source, args.last().unwrap(), "session")
            } else {
                String::new()
            };
            let parentheses = node
                .child_by_field_name("arguments")
                .is_some_and(|n| source.as_bytes()[n.start_byte()] == b'(');
            let replacement = if parentheses {
                format!(
                    "{}({action}{params}{session})",
                    String::from_utf8_lossy(&verb)
                )
            } else {
                format!(
                    "{} {action}{params}{session}",
                    String::from_utf8_lossy(&verb)
                )
            };
            d.corrected = push_replace(
                &mut corrections,
                node.start_byte(),
                send_end(node),
                replacement,
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
