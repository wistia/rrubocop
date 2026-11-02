use super::*;
use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, CopRegistry, EnabledState};
use crate::parse::source::SourceFile;

const CASES: &[&str] = &[
    "A, B = values\n",
    "(A, (B, C)), D = values\n",
    "A, (B, C), D = values\n",
    "A::B, ::C::D = values\n",
    "A, *B = values\n",
    "for A, B in rows; work; end\n",
    "A ||= 1; B += 1; C = 2\n",
    "class A; module B; C = value; end; end\n",
    "class A𐐀; A::B; C::D::E; end\n",
    "A::B; ::C::D; values = [A, B, C]\n",
];

fn old_definition(n: Node<'_>) -> bool {
    match n.kind() {
        "class" | "module" => true,
        "assignment" | "operator_assignment" => field(n, "left").is_some_and(constant),
        "constant" | "scope_resolution" => n.parent().is_some_and(|p| {
            matches!(
                p.kind(),
                "left_assignment_list" | "destructured_left_assignment"
            )
        }),
        _ => false,
    }
}
fn source(text: &str) -> SourceFile {
    SourceFile::from_bytes(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/ported_project/app/models/known.rb"
        ),
        text.as_bytes().to_vec(),
    )
}
fn positions<'n>(nodes: impl Iterator<Item = Node<'n>>) -> Vec<(usize, usize)> {
    let mut positions = nodes
        .map(|n| (n.start_byte(), n.end_byte()))
        .collect::<Vec<_>>();
    positions.sort();
    positions
}
#[test]
fn parent_dispatch_matches_the_previous_target_set_without_reference_callbacks() {
    let registry = CopRegistry::default_registry();
    let cop = registry
        .get("Cops/NoUndiscoverableAutoLoadedConstants")
        .unwrap();
    assert_eq!(
        cop.interested_parent_node_kinds("constant"),
        ["left_assignment_list", "destructured_left_assignment"]
    );
    for text in CASES {
        compare_targets(cop, text);
    }
}
fn compare_targets(cop: &dyn crate::cop::Cop, text: &str) {
    let source = source(text);
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let root = tree.root_node();
    assert_eq!(
        positions(descendants(root).filter(|n| old_definition(*n))),
        selected_positions(cop, root),
        "{text}"
    );
}
fn selected_positions(cop: &dyn crate::cop::Cop, root: Node<'_>) -> Vec<(usize, usize)> {
    positions(
        descendants(root)
            .filter(|n| cop.interested_node_kinds().contains(&n.kind()))
            .filter(|n| allowed_parent(cop, *n))
            .filter(|n| definition(*n)),
    )
}

fn allowed_parent(cop: &dyn crate::cop::Cop, n: Node<'_>) -> bool {
    let kinds = cop.interested_parent_node_kinds(n.kind());
    kinds.is_empty()
        || n.parent()
            .is_some_and(|parent| kinds.contains(&parent.kind()))
}

fn configuration() -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.target_ruby_version = Some(3.4);
    config.cop_configs.insert(
        "Cops/NoUndiscoverableAutoLoadedConstants".into(),
        CopConfig {
            enabled: EnabledState::True,
            include: vec!["**/*".into()],
            ..CopConfig::default()
        },
    );
    config
}
fn old_findings(source: &SourceFile) -> Vec<(usize, usize, usize)> {
    let tree = crate::parse::parse_ruby(source).unwrap();
    let config = CopConfig::default();
    let c = Context::new(source, &config);
    descendants(tree.root_node())
        .filter(|n| old_definition(*n))
        .filter_map(|n| inspect(&c, n))
        .map(|f| finding_position(source, f))
        .collect::<Vec<_>>()
}
fn finding_position(source: &SourceFile, finding: Finding) -> (usize, usize, usize) {
    let (line, column) = source.offset_to_line_col(finding.start);
    (line, column, display_width(source, &finding))
}

fn display_width(source: &SourceFile, finding: &Finding) -> usize {
    std::str::from_utf8(&source.content[finding.start..finding.end])
        .unwrap()
        .lines()
        .next()
        .unwrap_or("")
        .chars()
        .count()
        .max(1)
}

#[test]
fn nested_and_scoped_targets_keep_raw_engine_order_and_ranges_in_all_modes() {
    let registry = CopRegistry::default_registry();
    for text in CASES {
        for mode in [
            AutocorrectMode::Off,
            AutocorrectMode::Safe,
            AutocorrectMode::All,
        ] {
            compare_pipeline(&registry, text, mode);
        }
    }
}
fn compare_pipeline(registry: &CopRegistry, text: &str, mode: AutocorrectMode) {
    let source = source(text);
    let config = configuration();
    let findings = crate::linter::lint_source(
        &source,
        &config,
        registry,
        &CopFilterSet::build(&config, registry),
        Some(&["Cops/NoUndiscoverableAutoLoadedConstants".into()]),
        &[],
        mode,
        false,
    )
    .unwrap();
    let actual = findings
        .diagnostics
        .iter()
        .map(|d| {
            assert!(!d.correctable && !d.corrected);
            (d.location.line, d.location.column, d.highlight_length)
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, old_findings(&source), "{mode:?} {text}");
    assert!(findings.corrections.is_none_or(|edits| edits.is_empty()));
}

#[test]
fn public_linter_retains_order_for_interleaved_nested_targets() {
    let project = tempfile::tempdir().unwrap();
    let path = project.path().join("app/models/known.rb");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, CASES[1]).unwrap();
    check_public_order(&path);
}
fn check_public_order(path: &std::path::Path) {
    use clap::Parser;
    let registry = CopRegistry::default_registry();
    let config = configuration();
    let args = crate::cli::Args::parse_from([
        "rrubocop",
        "--cache",
        "false",
        "--only",
        "Cops/NoUndiscoverableAutoLoadedConstants",
    ]);
    let result = crate::linter::run_linter(&args, &config, &registry, &[path.to_owned()]).unwrap();
    let actual = result
        .diagnostics
        .iter()
        .map(|d| (d.location.line, d.location.column, d.highlight_length))
        .collect::<Vec<_>>();
    assert_eq!(actual, old_findings(&SourceFile::from_path(path).unwrap()));
}
