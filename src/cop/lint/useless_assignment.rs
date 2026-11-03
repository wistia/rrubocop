//! Lint/UselessAssignment.
mod captures;
mod flow;
mod similarity;
use crate::cop::shared::{node_bytes, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::model::{IntroKind, WriteKind};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct UselessAssignment;
fn sequential(node: Node<'_>) -> bool {
    let mut current = Some(node);
    while let Some(n) = current {
        if n.kind() == "assignment"
            && n.child_by_field_name("right").is_some_and(|rhs| {
                if !matches!(rhs.kind(), "array" | "right_assignment_list") {
                    return false;
                }
                let mut assignment = false;
                crate::cop::shared::for_each_descendant(rhs, |child| {
                    assignment |= matches!(child.kind(), "assignment" | "operator_assignment")
                });
                assignment
            })
        {
            return true;
        }
        current = n.parent();
    }
    false
}
fn last_value(node: Node<'_>) -> Option<Node<'_>> {
    let body = node.child_by_field_name("body").unwrap_or(node);
    if matches!(body.kind(), "program" | "body_statement" | "block_body") {
        let mut cursor = body.walk();
        body.named_children(&mut cursor)
            .filter(|n| n.kind() != "comment")
            .last()
    } else {
        Some(body)
    }
}
fn pattern_binding(node: Node<'_>) -> bool {
    let mut current = node.parent();
    while let Some(n) = current {
        if matches!(
            n.kind(),
            "keyword_pattern" | "array_pattern" | "hash_pattern" | "as_pattern"
        ) {
            return true;
        }
        if matches!(
            n.kind(),
            "assignment" | "operator_assignment" | "program" | "body_statement" | "block_body"
        ) {
            return false;
        }
        current = n.parent();
    }
    false
}
fn multiple_hint(node: Node<'_>) -> bool {
    let mut current = node.parent();
    while let Some(n) = current {
        if n.kind() == "rest_assignment" {
            return false;
        }
        if matches!(
            n.kind(),
            "left_assignment_list"
                | "destructured_left_assignment_list"
                | "destructured_left_assignment"
        ) {
            let mut cursor = n.walk();
            if n.named_children(&mut cursor)
                .any(|child| child.kind() == "rest_assignment")
            {
                return false;
            }
        }
        if n.kind() == "assignment" {
            return true;
        }
        if n.kind() == "for" {
            return false;
        }
        current = n.parent();
    }
    false
}
impl Cop for UselessAssignment {
    fn name(&self) -> &'static str {
        "Lint/UselessAssignment"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn needs_file_model(&self) -> bool {
        true
    }
    fn check_file_model(
        &self,
        source: &SourceFile,
        model: &crate::model::FileModel<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let original_index = flow::Index::new(model, model.tree.root_node());
        let augmented = captures::augment(source, model, &original_index);
        let captures = augmented
            .as_ref()
            .map(|(_, captures)| captures.as_slice())
            .unwrap_or_default();
        let model = augmented.as_ref().map(|(model, _)| model).unwrap_or(model);
        let mut index = if augmented.is_some() {
            flow::Index::new(model, model.tree.root_node())
        } else {
            original_index
        };
        for capture in captures {
            index.register_capture(capture.regex, capture.expression);
        }

        let mut ignored = Vec::new();
        for (scope_id, scope) in model.scopes.iter().enumerate() {
            let mut entries: Vec<_> = scope.entries.iter().collect();
            entries.sort_by_key(|(name, entry)| (entry.intro_byte, *name));
            let names = std::cell::OnceCell::new();
            for (name, entry) in entries {
                if name.starts_with('_') {
                    continue;
                }
                let mut unused = index.unused(source, model, scope_id, entry);
                unused.sort_by_key(|write| std::cmp::Reverse(index.write_rank(write)));
                for write in unused {
                    let Some(node) = index.nodes.get(&write.node_id).copied() else {
                        continue;
                    };
                    if pattern_binding(node)
                        || ignored.iter().any(|(start, end)| {
                            *start <= node.start_byte() && node.end_byte() <= *end
                        })
                    {
                        continue;
                    }
                    let capture = captures.iter().find(|capture| {
                        capture.scope == scope_id
                            && capture.regex == write.node_id
                            && capture.name == name.as_ref()
                    });
                    let (line, column) = source.offset_to_line_col(write.byte);
                    let parent = node.parent();
                    let mut message = format!("Useless assignment to variable - `{name}`.");
                    if write.kind == WriteKind::Masgn && multiple_hint(node) {
                        message += &format!(
                            " Use `_` or `_{name}` as a variable name to indicate that it won't be used."
                        );
                    }
                    if write.kind == WriteKind::OpAssign
                        && parent == last_value(index.scope_node(scope_id))
                    {
                        if let Some(operator) =
                            parent.and_then(|p| p.child_by_field_name("operator"))
                        {
                            let op = String::from_utf8_lossy(node_bytes(source, operator));
                            message += &format!(
                                " Use `{}` instead of `{op}`.",
                                op.strip_suffix('=').unwrap_or(&op)
                            );
                        }
                    }
                    if write.kind != WriteKind::OpAssign
                        && !(write.kind == WriteKind::Masgn && multiple_hint(node))
                    {
                        let names = names.get_or_init(|| index.names(source, model, scope_id));
                        if let Some(similar) = similarity::suggestion(name, names) {
                            message += &format!(" Did you mean `{similar}`?");
                        }
                    }
                    let mut diagnostic = self.diagnostic(source, line, column, message);
                    diagnostic.highlight_length = if capture.is_some() {
                        String::from_utf8_lossy(node_bytes(source, node))
                            .chars()
                            .count()
                    } else {
                        name.chars().count()
                    };
                    if let Some(capture) = capture {
                        let prefix = format!("(?<{name}>");
                        let raw = node_bytes(source, node);
                        if capture.backreference {
                            diagnostic.correctable = false;
                        } else if let Ok(raw) = std::str::from_utf8(raw) {
                            let replacement = raw.replacen(&prefix, "(?:", 1);
                            if replacement == raw {
                                diagnostic.correctable = false;
                            } else {
                                diagnostic.corrected = push_replace(
                                    &mut corrections,
                                    node.start_byte(),
                                    node.end_byte(),
                                    replacement,
                                    self.name(),
                                );
                            }
                        } else {
                            diagnostic.correctable = false;
                        }
                    } else if sequential(node) {
                        diagnostic.correctable = false;
                    } else if matches!(write.kind, WriteKind::Masgn | WriteKind::ForVar) {
                        diagnostic.corrected = push_replace(
                            &mut corrections,
                            node.start_byte(),
                            node.end_byte(),
                            "_",
                            self.name(),
                        );
                    } else if write.kind == WriteKind::RescueVar {
                        let rescue = parent.and_then(|p| p.parent());
                        if let Some(rescue) = rescue.filter(|n| n.kind() == "rescue") {
                            let start = rescue
                                .child_by_field_name("exceptions")
                                .map_or(rescue.start_byte() + 6, |exceptions| {
                                    exceptions.end_byte()
                                });
                            let drops_comment = crate::cop::shared::collect_comments(rescue)
                                .iter()
                                .any(|comment| {
                                    comment.start_byte() >= start
                                        && comment.end_byte() <= node.end_byte()
                                });
                            if drops_comment {
                                diagnostic.correctable = false;
                            } else {
                                diagnostic.corrected = push_replace(
                                    &mut corrections,
                                    start,
                                    node.end_byte(),
                                    "",
                                    self.name(),
                                );
                            }
                        } else {
                            diagnostic.correctable = false;
                        }
                    } else if write.kind == WriteKind::OpAssign {
                        if entry.intro_byte == write.byte && entry.intro_kind != IntroKind::Param {
                            diagnostic.correctable = false;
                        } else if let Some(operator) =
                            parent.and_then(|p| p.child_by_field_name("operator"))
                        {
                            diagnostic.corrected = push_replace(
                                &mut corrections,
                                operator.end_byte() - 1,
                                operator.end_byte(),
                                "",
                                self.name(),
                            );
                        }
                    } else if let Some(assignment) = parent.filter(|p| p.kind() == "assignment") {
                        if let Some(rhs) = assignment.child_by_field_name("right") {
                            let drops_comment = crate::cop::shared::collect_comments(assignment)
                                .iter()
                                .any(|comment| {
                                    comment.start_byte() < rhs.start_byte()
                                        || comment.end_byte() > rhs.end_byte()
                                });
                            let self_declaration = entry.intro_byte == write.byte
                                && entry.reads.iter().any(|read| {
                                    rhs.start_byte() <= read.byte && read.byte < rhs.end_byte()
                                })
                                && entry.writes.iter().all(|other| other.byte >= write.byte);
                            if drops_comment || self_declaration {
                                diagnostic.correctable = false;
                            } else if let Ok(replacement) =
                                std::str::from_utf8(node_bytes(source, rhs))
                            {
                                diagnostic.corrected = push_replace(
                                    &mut corrections,
                                    assignment.start_byte(),
                                    assignment.end_byte(),
                                    replacement,
                                    self.name(),
                                );
                            } else {
                                diagnostic.corrected = push_replace(
                                    &mut corrections,
                                    assignment.start_byte(),
                                    rhs.start_byte(),
                                    "",
                                    self.name(),
                                );
                                if rhs.end_byte() < assignment.end_byte() {
                                    push_replace(
                                        &mut corrections,
                                        rhs.end_byte(),
                                        assignment.end_byte(),
                                        "",
                                        self.name(),
                                    );
                                }
                            }
                            if rhs.kind() == "assignment" {
                                ignored.push((assignment.start_byte(), assignment.end_byte()));
                            }
                        } else {
                            diagnostic.correctable = false;
                        }
                    }
                    diagnostics.push(diagnostic);
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_fresh_local_declarations_for_operator_assignments() {
        let mut fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/useless_assignment/intentional-safeguards.json"
        )))
        .unwrap();
        for case in fixture.as_array_mut().unwrap() {
            case["ruby_corrected"] = case["rust_corrected"].clone();
        }
        crate::testutil::assert_original_spec_parity(&UselessAssignment, &fixture.to_string());
    }
    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &UselessAssignment,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/useless_assignment/parity.json"
            )),
        );
    }
    crate::cop_fixture_tests!(UselessAssignment, "cops/lint/useless_assignment");
}
