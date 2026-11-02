use crate::cop::rspec::description;
use crate::cop::rspec::helpers::{RSPEC_INCLUDE, is_example};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn excessive(text: &str) -> bool {
    let chars: Vec<_> = text.chars().collect();
    if chars.first().copied().is_some_and(description::blank)
        || chars.last().copied().is_some_and(description::blank)
    {
        return true;
    }
    for start in 1..chars.len() {
        if description::blank(chars[start]) && !chars[start - 1].is_whitespace() {
            let count = chars[start..]
                .iter()
                .take_while(|character| description::blank(**character))
                .count();
            if count >= 2
                && chars
                    .get(start + count)
                    .is_some_and(|character| !description::blank(*character))
            {
                return true;
            }
        }
    }
    false
}
fn normalize(text: &str) -> String {
    let mut chars = text.chars().peekable();
    let mut normalized = String::new();
    while let Some(character) = chars.next() {
        if description::blank(character)
            && chars.peek().is_some_and(|next| description::blank(*next))
        {
            while chars.peek().is_some_and(|next| description::blank(*next)) {
                chars.next();
            }
            normalized.push(' ');
        } else {
            normalized.push(character);
        }
    }
    normalized.trim_matches(description::blank).into()
}
pub struct ExcessiveDocstringSpacing;
impl Cop for ExcessiveDocstringSpacing {
    fn name(&self) -> &'static str {
        "RSpec/ExcessiveDocstringSpacing"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if safe_send(source, node)
            || !name(source, node).is_some_and(|name| is_example(name) || is_example_group(name))
        {
            return;
        }
        let Some(description) = call_args(node).first().copied() else {
            return;
        };
        if description.kind() == "heredoc_beginning"
            || !description::first_static(source, description, true)
        {
            return;
        }
        let Some(text) = description::text(source, description).filter(|text| excessive(text))
        else {
            return;
        };
        let start = description.start_byte() + 1;
        let end = description.end_byte().saturating_sub(1).max(start);
        let index = report(
            self,
            source,
            start,
            end,
            "Excessive whitespace.".into(),
            diagnostics,
        );
        let replacement = normalize(&text);
        if description::safe_replacement(source, description, &replacement)
            && preserving_replace(
                source,
                description,
                &mut corrections,
                start,
                end,
                replacement,
                self.name(),
            )
        {
            diagnostics[index].corrected = true;
        }
    }
}
