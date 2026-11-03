use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

struct RoutedCop<'a> {
    inner: &'a dyn Cop,
    gated: bool,
    callbacks: AtomicUsize,
}

impl Cop for RoutedCop<'_> {
    fn name(&self) -> &'static str {
        self.inner.name()
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        self.inner.interested_node_kinds()
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        if self.gated {
            self.inner.interested_call_names()
        } else {
            &[]
        }
    }
    fn interested_identifier_names(&self) -> &'static [&'static [u8]] {
        if self.gated {
            self.inner.interested_identifier_names()
        } else {
            &[]
        }
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<Correction>>,
    ) {
        self.callbacks.fetch_add(1, Ordering::Relaxed);
        self.inner
            .check_node(source, node, config, diagnostics, corrections);
    }
}

fn inspect(
    cop: &dyn Cop,
    gated: bool,
    code: &[u8],
    config: &CopConfig,
    correct: bool,
) -> (serde_json::Value, String, Vec<u8>, usize) {
    let source = SourceFile::from_bytes("spec/routing_spec.rb", code.to_vec());
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let routed = RoutedCop {
        inner: cop,
        gated,
        callbacks: AtomicUsize::new(0),
    };
    let mut diagnostics = Vec::new();
    let mut corrections = Vec::new();
    BatchedWalker::new(vec![&routed], vec![config]).walk(
        &source,
        tree.root_node(),
        &mut diagnostics,
        correct.then_some(&mut corrections),
    );
    let edits = format!("{corrections:?}");
    let bytes = crate::correction::CorrectionSet::from_vec(corrections).apply(code);
    (
        serde_json::to_value(diagnostics).unwrap(),
        edits,
        bytes,
        routed.callbacks.load(Ordering::Relaxed),
    )
}

fn assert_routing(cop: &dyn Cop, cases: &[serde_json::Value]) -> (usize, usize) {
    let mut counts = (0, 0);
    for case in cases {
        let code = case
            .get("original")
            .or_else(|| case.get("source"))
            .unwrap()
            .as_str()
            .unwrap();
        let config = CopConfig {
            options: case["config"]
                .as_object()
                .map(|object| {
                    object
                        .iter()
                        .map(|(key, value)| (key.clone(), serde_yml::to_value(value).unwrap()))
                        .collect()
                })
                .unwrap_or_default(),
            ..CopConfig::default()
        };
        for correct in [false, true] {
            let baseline = inspect(cop, false, code.as_bytes(), &config, correct);
            let gated = inspect(cop, true, code.as_bytes(), &config, correct);
            assert_eq!(baseline.0, gated.0, "{}: {}", cop.name(), case["id"]);
            assert_eq!(baseline.1, gated.1, "{}: {}", cop.name(), case["id"]);
            assert_eq!(baseline.2, gated.2, "{}: {}", cop.name(), case["id"]);
            let final_baseline = inspect(cop, false, &baseline.2, &config, correct);
            let final_gated = inspect(cop, true, &gated.2, &config, correct);
            assert_eq!(
                (&final_baseline.0, &final_baseline.1, &final_baseline.2),
                (&final_gated.0, &final_gated.1, &final_gated.2)
            );
            counts.0 += baseline.3;
            counts.1 += gated.3;
        }
    }
    counts
}

#[test]
fn hot_cop_routing_preserves_original_corpora_and_guard_states() {
    let registry = crate::cop::CopRegistry::default_registry();
    let rspec: Vec<serde_json::Value> = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/rspec-parity/cases.json"
    )))
    .unwrap();
    for name in [
        "RSpec/Focus",
        "RSpec/PendingWithoutReason",
        "Style/NonNilCheck",
    ] {
        let cases = if name == "Style/NonNilCheck" {
            let mut cases: Vec<serde_json::Value> = serde_json::from_str(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/non_nil_check/parity.json"
            )))
            .unwrap();
            cases.extend(
                serde_json::from_str::<Vec<serde_json::Value>>(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/tests/fixtures/cops/style/non_nil_check/intentional-safeguards.json"
                )))
                .unwrap(),
            );
            cases
        } else {
            rspec
                .iter()
                .filter(|case| case["cop"] == name)
                .cloned()
                .collect()
        };
        assert!(!cases.is_empty());
        let counts = assert_routing(registry.get(name).unwrap(), &cases);
        assert!(counts.1 < counts.0, "{name}: routing must reduce callbacks");
        eprintln!(
            "{name}: {} sources; callbacks {} -> {}",
            cases.len(),
            counts.0,
            counts.1
        );
    }
}

#[test]
fn hot_cop_routing_skips_irrelevant_calls_and_identifiers() {
    let registry = crate::cop::CopRegistry::default_registry();
    let code = "object.ordinary(value)\n".repeat(1000);
    for name in [
        "RSpec/Focus",
        "RSpec/PendingWithoutReason",
        "Style/NonNilCheck",
    ] {
        let cop = registry.get(name).unwrap();
        let baseline = inspect(cop, false, code.as_bytes(), &CopConfig::default(), true);
        let gated = inspect(cop, true, code.as_bytes(), &CopConfig::default(), true);
        assert_eq!(baseline.0, gated.0);
        assert_eq!(baseline.1, gated.1);
        assert_eq!(baseline.2, gated.2);
        assert!(baseline.3 >= 1000);
        assert_eq!(gated.3, 0);
    }
}
