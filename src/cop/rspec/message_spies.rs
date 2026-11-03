use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::{call_receiver, node_text};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct MessageSpies;
impl Cop for MessageSpies {
    fn name(&self) -> &'static str {
        "RSpec/MessageSpies"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if !runner(source, node) {
            return;
        }
        let Some(expectation) = call_receiver(node) else {
            return;
        };
        if !bare(source, expectation, b"expect")
            || safe_send(source, expectation)
            || call_block(expectation).is_some()
        {
            return;
        }
        let targets = call_args(expectation);
        if targets.len() != 1
            && (targets.is_empty()
                || targets
                    .iter()
                    .any(|target| !matches!(target.kind(), "pair" | "hash_splat_argument")))
        {
            return;
        }
        let style = config.get_str("EnforcedStyle", "have_received");
        let target = if targets.len() == 1 {
            node_text(source, targets[0])
        } else {
            String::from_utf8_lossy(
                &source.as_bytes()[targets[0].start_byte()..targets.last().unwrap().end_byte()],
            )
            .into_owned()
        };
        let mut pending = Vec::new();
        let mut root_children = vec![expectation];
        root_children.extend(call_args(node));
        pending.extend(root_children.into_iter().rev());
        while let Some(child) = pending.pop() {
            if !safe_send(source, child)
                && call_receiver(child).is_none()
                && matches!(name(source, child), Some(b"receive" | b"have_received"))
            {
                if name(source, child) == Some(style.as_bytes()) {
                    break;
                }
                let selector = selector(child);
                let message = if style == "receive" {
                    "Prefer `receive` for setting message expectations.".into()
                } else {
                    format!(
                        "Prefer `have_received` for setting message expectations. Setup `{target}` as a spy using `allow` or `instance_spy`."
                    )
                };
                report(
                    self,
                    source,
                    selector.start_byte(),
                    selector.end_byte(),
                    message,
                    diagnostics,
                );
            }
            pending.extend(code_children(child).into_iter().rev());
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(MessageSpies, "cops/rspec/message_spies");

    #[test]
    fn receive_style_flags_have_received() {
        let mut config = CopConfig::default();
        config.options.insert(
            "EnforcedStyle".into(),
            serde_yml::Value::String("receive".into()),
        );
        crate::testutil::assert_cop_offenses_full_with_config(
            &MessageSpies,
            b"expect(foo).to have_received(:bar)\n               ^^^^^^^^^^^^^ RSpec/MessageSpies: Prefer `receive` for setting message expectations.\n",
            config,
        );
    }
}
