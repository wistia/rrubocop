//! RuboCop `on_class` / `on_module` / `on_sclass`: each body member sits `Width`
//! columns past the keyword (or past `private`/`protected` in
//! `indented_internal_methods` style). The line scan cannot see an outdented body.

use tree_sitter::Node;

use crate::cop::shared;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

const MODIFIERS: &[&[u8]] = &[b"private", b"protected", b"public", b"module_function"];

/// A misindented member and its column offset from its base.
pub(super) struct Offense<'a> {
    node: Node<'a>,
    indentation: isize,
}

struct Ctx<'s> {
    source: &'s SourceFile,
    width: isize,
    internal: bool,
    outdent: bool,
}

fn modifier_name<'a>(source: &'a SourceFile, node: Node<'_>) -> Option<&'a [u8]> {
    let name = match node.kind() {
        "identifier" => shared::node_bytes(source, node),
        "call" | "command" if shared::call_receiver(node).is_none() => {
            shared::call_method_name(source, node)?
        }
        _ => return None,
    };
    MODIFIERS.contains(&name).then_some(name)
}

fn members(body: Node<'_>) -> Vec<Node<'_>> {
    let mut cur = body.walk();
    body.named_children(&mut cur)
        .filter(|n| !matches!(n.kind(), "comment" | "rescue" | "else" | "ensure"))
        .collect()
}

fn col(ctx: &Ctx<'_>, node: Node<'_>) -> isize {
    shared::node_col(ctx.source, node) as isize
}

fn check_member<'a>(ctx: &Ctx<'_>, member: Node<'a>, base: isize, out: &mut Vec<Offense<'a>>) {
    let member_col = shared::node_col(ctx.source, member);
    if member_col != shared::line_indent(ctx.source, member.start_byte()) {
        return;
    }
    let indentation = member_col as isize - base;
    if indentation != ctx.width {
        out.push(Offense {
            node: member,
            indentation,
        });
    }
}

/// Base column for the member after `name`: only `private`/`protected` re-anchor
/// in `indented_internal_methods` style; normal style always uses the keyword.
fn next_base(ctx: &Ctx<'_>, member: Node<'_>, name: &[u8], kw: isize) -> Option<isize> {
    if !ctx.internal {
        return Some(kw);
    }
    matches!(name, b"private" | b"protected").then(|| col(ctx, member))
}

/// Check one member against `base`; returns the base for the next member.
fn visit<'a>(
    ctx: &Ctx<'_>,
    member: Node<'a>,
    first: bool,
    kw: isize,
    base: Option<isize>,
    out: &mut Vec<Offense<'a>>,
) -> Option<isize> {
    if let Some(name) = modifier_name(ctx.source, member) {
        if first && !ctx.outdent {
            check_member(ctx, member, kw, out);
        }
        return next_base(ctx, member, name, kw);
    }
    if let Some(b) = base {
        check_member(ctx, member, b, out);
    }
    base.filter(|_| !ctx.internal)
}

fn body_offenses<'a>(ctx: &Ctx<'_>, node: Node<'a>, out: &mut Vec<Offense<'a>>) {
    let Some(members) = node.child_by_field_name("body").map(members) else {
        return;
    };
    let kw_line = shared::node_line(ctx.source, node);
    if members
        .first()
        .is_none_or(|m| shared::node_line(ctx.source, *m) == kw_line)
    {
        return;
    }
    let kw = col(ctx, node);
    let mut base = Some(kw);
    for (i, member) in members.into_iter().enumerate() {
        base = visit(ctx, member, i == 0, kw, base, out);
    }
}

fn offenses<'a>(
    source: &SourceFile,
    root: Node<'a>,
    width: usize,
    config: &CopConfig,
) -> Vec<Offense<'a>> {
    let ctx = Ctx {
        source,
        width: width as isize,
        internal: config.get_str("IndentationConsistencyStyle", "normal")
            == "indented_internal_methods",
        outdent: config.get_str("AccessModifierIndentationStyle", "indent") == "outdent",
    };
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if matches!(node.kind(), "class" | "module" | "singleton_class") {
            body_offenses(&ctx, node, &mut out);
        }
        let mut cur = node.walk();
        stack.extend(node.named_children(&mut cur));
    }
    out
}

/// Run the line `scan`, drop its reports on lines a body offense owns, then add
/// the body offenses (RuboCop's signed width and whole-member shift).
pub(super) fn check(
    cop: &dyn Cop,
    source: &SourceFile,
    root: Node<'_>,
    code_map: &CodeMap,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<Correction>>,
    scan: impl FnOnce(&mut Vec<Diagnostic>, &mut Option<&mut Vec<Correction>>),
) {
    let width = config.get_usize("Width", 2);
    let owned = offenses(source, root, width, config);
    let lines: Vec<usize> = owned
        .iter()
        .map(|o| shared::node_line(source, o.node))
        .collect();
    let (mut scan_diags, mut scan_corr) = (Vec::new(), Vec::new());
    scan(
        &mut scan_diags,
        &mut corrections.is_some().then_some(&mut scan_corr),
    );
    keep_unowned(
        source,
        &lines,
        (scan_diags, scan_corr),
        diagnostics,
        &mut corrections,
    );
    for offense in &owned {
        report(
            cop,
            source,
            code_map,
            width,
            offense,
            diagnostics,
            &mut corrections,
        );
    }
}

fn keep_unowned(
    source: &SourceFile,
    lines: &[usize],
    (scan_diags, scan_corr): (Vec<Diagnostic>, Vec<Correction>),
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    diagnostics.extend(
        scan_diags
            .into_iter()
            .filter(|d| !lines.contains(&d.location.line)),
    );
    if let Some(corr) = corrections {
        corr.extend(
            scan_corr
                .into_iter()
                .filter(|c| !lines.contains(&source.offset_to_line_col(c.start).0)),
        );
    }
}

fn report(
    cop: &dyn Cop,
    source: &SourceFile,
    code_map: &CodeMap,
    width: usize,
    offense: &Offense<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (node, indentation) = (offense.node, offense.indentation);
    let member_col = shared::node_col(source, node);
    let col = if indentation >= 0 {
        member_col - indentation as usize
    } else {
        member_col
    };
    let message = format!("Use {width} (not {indentation}) spaces for indentation.");
    let mut diag = cop.diagnostic(source, shared::node_line(source, node), col, message);
    diag.highlight_length = indentation.unsigned_abs().max(1);
    if let Some(corr) = corrections
        && crate::cop::layout::report::shift_lines(
            cop,
            source,
            node,
            width as isize - indentation,
            |start| code_map.covers(start),
            corr,
        )
    {
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::layout::indentation_width::IndentationWidth;
    use crate::correction::CorrectionSet;

    fn corrected(src: &str) -> Vec<u8> {
        let source = SourceFile::from_bytes("t.rb", src.into());
        let tree = crate::parse::parse_ruby(&source).expect("parse");
        let mut corrections = Vec::new();
        IndentationWidth.check_source(
            &source,
            &tree,
            &CodeMap::from_tree(tree.root_node(), source.as_bytes()),
            &CopConfig::default(),
            &mut vec![],
            Some(&mut corrections),
        );
        CorrectionSet::from_vec(corrections).apply(source.as_bytes())
    }

    #[test]
    fn autocorrect_shifts_outdented_class_member() {
        assert_eq!(
            corrected("module M\n  class C\n  X = f(\n    1,\n  )\n\nY = <<~T\n  text\nT\n  end\nend\n"),
            b"module M\n  class C\n    X = f(\n      1,\n    )\n\n    Y = <<~T\n  text\nT\n  end\nend\n",
        );
    }
}
