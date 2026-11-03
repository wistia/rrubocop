//! Enforce the configured HTTP status representation using Wistia's pinned Rack status names.
use crate::cop::shared::ruby_expression::{integer_ast, plain_string, plain_symbol};
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, named_kids, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct HttpStatus;
const STATUS: &[(&str, i32)] = &[
    ("continue", 100),
    ("switching_protocols", 101),
    ("processing", 102),
    ("early_hints", 103),
    ("ok", 200),
    ("created", 201),
    ("accepted", 202),
    ("non_authoritative_information", 203),
    ("no_content", 204),
    ("reset_content", 205),
    ("partial_content", 206),
    ("multi_status", 207),
    ("already_reported", 208),
    ("im_used", 226),
    ("multiple_choices", 300),
    ("moved_permanently", 301),
    ("found", 302),
    ("see_other", 303),
    ("not_modified", 304),
    ("use_proxy", 305),
    ("(unused)", 306),
    ("temporary_redirect", 307),
    ("permanent_redirect", 308),
    ("bad_request", 400),
    ("unauthorized", 401),
    ("payment_required", 402),
    ("forbidden", 403),
    ("not_found", 404),
    ("method_not_allowed", 405),
    ("not_acceptable", 406),
    ("proxy_authentication_required", 407),
    ("request_timeout", 408),
    ("conflict", 409),
    ("gone", 410),
    ("length_required", 411),
    ("precondition_failed", 412),
    ("payload_too_large", 413),
    ("uri_too_long", 414),
    ("unsupported_media_type", 415),
    ("range_not_satisfiable", 416),
    ("expectation_failed", 417),
    ("misdirected_request", 421),
    ("unprocessable_entity", 422),
    ("locked", 423),
    ("failed_dependency", 424),
    ("too_early", 425),
    ("upgrade_required", 426),
    ("precondition_required", 428),
    ("too_many_requests", 429),
    ("request_header_fields_too_large", 431),
    ("unavailable_for_legal_reasons", 451),
    ("internal_server_error", 500),
    ("not_implemented", 501),
    ("bad_gateway", 502),
    ("service_unavailable", 503),
    ("gateway_timeout", 504),
    ("http_version_not_supported", 505),
    ("variant_also_negotiates", 506),
    ("insufficient_storage", 507),
    ("loop_detected", 508),
    ("bandwidth_limit_exceeded", 509),
    ("not_extended", 510),
    ("network_authentication_required", 511),
];
fn decimal_prefix(value: &str) -> num_bigint::BigInt {
    let value = value.trim_start_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']);
    let (negative, value) = if let Some(value) = value.strip_prefix('-') {
        (true, value)
    } else {
        (false, value.strip_prefix('+').unwrap_or(value))
    };
    let mut digits = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else if ch != '_' || digits.is_empty() || !chars.peek().is_some_and(char::is_ascii_digit)
        {
            break;
        }
    }
    let value = num_bigint::BigInt::parse_bytes(digits.as_bytes(), 10).unwrap_or_default();
    if negative { -value } else { value }
}
fn status_value<'a>(
    source: &SourceFile,
    pairs: impl IntoIterator<Item = Node<'a>>,
) -> Option<Node<'a>> {
    pairs.into_iter().find_map(|pair| {
        if pair.kind() != "pair"
            || plain_symbol(source, pair.child_by_field_name("key")?).as_deref() != Some("status")
        {
            return None;
        }
        let value = pair.child_by_field_name("value")?;
        (integer_ast(source, value).is_some()
            || plain_symbol(source, value).is_some()
            || plain_string(source, value).is_some())
        .then_some(value)
    })
}
impl Cop for HttpStatus {
    fn name(&self) -> &'static str {
        "Rails/HttpStatus"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[
            b"render",
            b"redirect_to",
            b"head",
            b"assert_response",
            b"assert_redirected_to",
        ]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !super::unknown_env::regular(node, source)
            || call_receiver(node).is_some()
            || crate::cop::shared::is_setter_call(node)
        {
            return;
        }
        let Some(method) = call_method_name(source, node) else {
            return;
        };
        let args = argument_nodes(node);
        let Some(first) = args.first().copied() else {
            return;
        };
        let status = if matches!(method, b"head" | b"assert_response") {
            (integer_ast(source, first).is_some() || plain_symbol(source, first).is_some())
                .then_some(first)
        } else if matches!(method, b"render" | b"redirect_to" | b"assert_redirected_to") {
            let mut semantic = Vec::new();
            let mut index = 0;
            while index < args.len() {
                if matches!(args[index].kind(), "pair" | "hash_splat_argument") {
                    semantic.push((None, args[index..].to_vec()));
                    break;
                }
                semantic.push((Some(args[index]), Vec::new()));
                index += 1;
            }
            if matches!(method, b"render" | b"redirect_to") && !(1..=2).contains(&semantic.len()) {
                return;
            }
            let is_hash = |index: usize| {
                semantic
                    .get(index)
                    .is_some_and(|(n, p)| !p.is_empty() || n.is_some_and(|n| n.kind() == "hash"))
            };
            let position = if is_hash(1) {
                1
            } else if (semantic.len() == 1 || method == b"assert_redirected_to") && is_hash(0) {
                0
            } else {
                return;
            };
            semantic.get(position).and_then(|(n, p)| {
                if let Some(hash) = n {
                    if hash.kind() == "hash" {
                        status_value(source, named_kids(*hash))
                    } else {
                        None
                    }
                } else {
                    status_value(source, p.iter().copied())
                }
            })
        } else {
            None
        };
        let Some(status) = status else {
            return;
        };
        let integer = integer_ast(source, status);
        let symbol = plain_symbol(source, status);
        let string = plain_string(source, status);
        let numeric = config.get_str("EnforcedStyle", "symbolic") == "numeric";
        let (preferred, current, correctable) = if numeric {
            if integer.is_some() {
                return;
            }
            let Some(symbol) = symbol else {
                return;
            };
            if matches!(
                symbol.as_str(),
                "error" | "success" | "missing" | "redirect"
            ) {
                return;
            }
            let Some((_, number)) = STATUS.iter().find(|(name, _)| *name == symbol) else {
                return;
            };
            (
                number.to_string(),
                crate::cop::shared::ruby_literal::symbol_inspect(&symbol),
                true,
            )
        } else {
            if symbol.is_some() {
                return;
            }
            let (number, current, is_string) = if let Some(integer) = integer {
                (integer.clone(), integer.to_string(), false)
            } else if let Some(string) = string {
                (decimal_prefix(&string), string, true)
            } else {
                return;
            };
            let mapped = STATUS
                .iter()
                .find(|(_, code)| num_bigint::BigInt::from(*code) == number);
            if mapped.is_none() && !is_string {
                return;
            }
            (
                mapped.map_or_else(
                    || "nil".into(),
                    |(name, _)| crate::cop::shared::ruby_literal::symbol_inspect(name),
                ),
                current,
                mapped.is_some(),
            )
        };
        let (line, col) = source.offset_to_line_col(status.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!("Prefer `{preferred}` over `{current}` to define HTTP status code."),
        );
        d.highlight_length =
            super::blank::highlight_length(source, status.start_byte(), status.end_byte());
        d.correctable = correctable && !config.autocorrect_disabled();
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                status.start_byte(),
                status.end_byte(),
                preferred,
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
