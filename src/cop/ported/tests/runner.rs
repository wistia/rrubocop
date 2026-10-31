use super::*;
use crate::cop::walker::BatchedWalker;
use crate::correction::{Correction, CorrectionSet};

pub(super) fn run(registry: &CopRegistry, case: &Case) -> (Vec<String>, String) {
    let source = source(case);
    let (diagnostics, corrections) = inspect(registry, case, &source);
    (
        actual(diagnostics),
        String::from_utf8(CorrectionSet::from_vec(corrections).apply(source.as_bytes())).unwrap(),
    )
}
fn source(case: &Case) -> SourceFile {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ported_project");
    SourceFile::from_bytes(
        case.path.replace("$PROJECT", root),
        case.source.as_bytes().to_vec(),
    )
}
fn inspect(
    registry: &CopRegistry,
    case: &Case,
    source: &SourceFile,
) -> (Vec<Diagnostic>, Vec<Correction>) {
    let tree = crate::parse::parse_ruby(source).unwrap();
    let config = CopConfig {
        options: case.config.clone(),
        ..CopConfig::default()
    };
    let mut diagnostics = vec![];
    let mut corrections = vec![];
    BatchedWalker::new(vec![registry.get(&case.cop).unwrap()], vec![&config]).walk(
        source,
        tree.root_node(),
        &mut diagnostics,
        Some(&mut corrections),
    );
    (diagnostics, corrections)
}
fn actual(diagnostics: Vec<Diagnostic>) -> Vec<String> {
    let mut actual = diagnostics
        .into_iter()
        .map(|d| {
            format!(
                "{}:{}:{}:{}",
                d.location.line, d.location.column, d.correctable, d.message
            )
        })
        .collect::<Vec<_>>();
    actual.sort();
    actual
}
pub(super) fn expected(case: &Case) -> Vec<String> {
    let mut values = case
        .expected
        .iter()
        .map(|e| {
            format!(
                "{}:{}:{}:{}",
                e.line,
                e.column,
                case.rust_correctable.unwrap_or(e.correctable),
                e.message
            )
        })
        .collect::<Vec<_>>();
    values.sort();
    values
}
pub(super) fn expected_correction(case: &Case) -> &str {
    case.rust_corrected.as_ref().unwrap_or(&case.corrected)
}
pub(super) fn assert_case(registry: &CopRegistry, case: &Case) {
    let (actual, corrected) = run(registry, case);
    assert_eq!(actual, expected(case), "{}", case.cop);
    assert_eq!(&corrected, expected_correction(case), "{}", case.cop);
}
