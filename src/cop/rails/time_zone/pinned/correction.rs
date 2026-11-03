use super::{
    ACCEPTED_METHODS, arguments, file_literal, regular_send_name, regular_send_receiver, selector,
    send_parent, target, time_constant,
};
use crate::cop::shared::ruby_expression::{
    float_value, integer_ast, plain_string, send_arguments, send_name,
};
use crate::cop::shared::ruby_semantics::value_used;
use crate::cop::shared::{collect_comments, named_kids, node_bytes};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(super) type Edits = Vec<(usize, usize, String)>;
#[derive(Clone, Copy)]
enum Action {
    Zone,
    Environment,
    Barrier,
}
struct Environment {
    overridden: bool,
    statements: Vec<(usize, Action)>,
}
fn root_constant(source: &SourceFile, node: Node<'_>, name: &[u8]) -> bool {
    (node.kind() == "constant" && node_bytes(source, node) == name)
        || (node.kind() == "scope_resolution"
            && node.child_by_field_name("scope").is_none()
            && node
                .child_by_field_name("name")
                .is_some_and(|token| node_bytes(source, token) == name))
}
fn utc_assignment(source: &SourceFile, node: Node<'_>) -> Option<Action> {
    let name = regular_send_name(source, node)?;
    let receiver = regular_send_receiver(source, node).flatten()?;
    let args = send_arguments(source, node)?;
    if name.as_ref() == b"zone="
        && time_constant(source, receiver)
        && matches!(args.as_slice(),[value] if plain_string(source,*value).as_deref()==Some("UTC"))
    {
        return Some(Action::Zone);
    }
    if name.as_ref() == b"[]="
        && root_constant(source, receiver, b"ENV")
        && matches!(args.as_slice(),[key,value] if plain_string(source,*key).as_deref()==Some("TZ") && plain_string(source,*value).as_deref()==Some("UTC"))
    {
        return Some(Action::Environment);
    }
    None
}
fn projected_lhs(node: Node<'_>) -> bool {
    node.parent().is_some_and(|parent| {
        parent.kind() == "assignment" && parent.child_by_field_name("left") == Some(node)
    })
}
fn sensitive_write(source: &SourceFile, node: Node<'_>) -> bool {
    if projected_lhs(node) {
        return false;
    }
    let Some(name) = send_name(source, node) else {
        return false;
    };
    let Some(receiver) = target(node)
        .child_by_field_name("receiver")
        .or_else(|| target(node).child_by_field_name("object"))
    else {
        return false;
    };
    (time_constant(source, receiver) && matches!(name.as_ref(), b"zone=" | b"zone_default="))
        || (root_constant(source, receiver, b"ENV")
            && matches!(
                name.as_ref(),
                b"[]="
                    | b"store"
                    | b"replace"
                    | b"update"
                    | b"merge!"
                    | b"clear"
                    | b"delete"
                    | b"delete_if"
                    | b"reject!"
                    | b"keep_if"
                    | b"select!"
                    | b"shift"
            ))
}
fn environment(source: &SourceFile, node: Node<'_>) -> std::sync::Arc<Environment> {
    source.cached(0, || {
        let mut root = node;
        while let Some(parent) = root.parent() {
            root = parent;
        }
        let statements = named_kids(root)
            .into_iter()
            .filter(|node| node.kind() != "comment")
            .map(|node| {
                (
                    node.start_byte(),
                    utc_assignment(source, node).unwrap_or(Action::Barrier),
                )
            })
            .collect();
        let mut pending = vec![root];
        let mut overridden = false;
        while let Some(node) = pending.pop() {
            if matches!(node.kind(), "method" | "singleton_method")
                && node.child_by_field_name("name").is_some_and(|name| {
                    matches!(
                        node_bytes(source, name),
                        b"now"
                            | b"new"
                            | b"local"
                            | b"parse"
                            | b"at"
                            | b"zone"
                            | b"zone="
                            | b"zone_default"
                            | b"to_time"
                            | b"to_i"
                            | b"to_f"
                            | b"in_time_zone"
                            | b"utc"
                            | b"getlocal"
                    )
                })
            {
                overridden = true;
            }
            if matches!(
                node.kind(),
                "assignment" | "operator_assignment" | "class" | "module"
            ) {
                let binding = node
                    .child_by_field_name("left")
                    .or_else(|| node.child_by_field_name("name"));
                if binding.is_some_and(|name| {
                    root_constant(source, name, b"Time") || root_constant(source, name, b"ENV")
                }) {
                    overridden = true;
                }
            }
            if sensitive_write(source, node)
                && !node
                    .parent()
                    .is_some_and(|parent| parent.kind() == "program")
            {
                overridden = true;
            }
            if node.kind() == "alias"
                || send_name(source, node).is_some_and(|name| {
                    matches!(
                        name.as_ref(),
                        b"alias_method"
                            | b"define_method"
                            | b"define_singleton_method"
                            | b"class_eval"
                            | b"module_eval"
                            | b"instance_eval"
                            | b"eval"
                            | b"prepend"
                            | b"set_trace_func"
                            | b"travel"
                            | b"travel_to"
                            | b"freeze_time"
                            | b"unfreeze_time"
                            | b"stub"
                            | b"stub_const"
                            | b"instance_variable_set"
                            | b"instance_variable_get"
                            | b"const_get"
                            | b"const_set"
                            | b"remove_const"
                            | b"send"
                            | b"public_send"
                            | b"__send__"
                            | b"autoload"
                    )
                })
            {
                overridden = true;
            }
            pending.extend(named_kids(node));
        }
        Environment {
            overridden,
            statements,
        }
    })
}
fn top_statement<'n>(source: &SourceFile, expression: Node<'n>) -> Option<Node<'n>> {
    let parent = expression.parent()?;
    if parent.kind() == "program" {
        return Some(expression);
    }
    if parent.kind() == "assignment"
        && parent.child_by_field_name("right") == Some(expression)
        && parent
            .child_by_field_name("left")
            .is_some_and(|left| left.kind() == "identifier")
        && parent.parent().is_some_and(|root| root.kind() == "program")
    {
        return Some(parent);
    }
    if parent.kind() != "argument_list" {
        return None;
    }
    let output = parent.parent()?;
    if regular_send_receiver(source, output)
        .flatten()
        .is_some_and(|receiver| !root_constant(source, receiver, b"Kernel"))
    {
        return None;
    }
    if !regular_send_name(source, output)
        .is_some_and(|name| matches!(name.as_ref(), b"p" | b"puts" | b"print"))
        || target(output).child_by_field_name("block").is_some()
        || !output.parent().is_some_and(|root| root.kind() == "program")
    {
        return None;
    }
    let args = arguments(source, output);
    (args.len() == 1 && args[0].node() == Some(expression)).then_some(output)
}
fn initialized(source: &SourceFile, node: Node<'_>, statement: Node<'_>) -> bool {
    let state = environment(source, node);
    if state.overridden {
        return false;
    }
    let (mut zone, mut environment) = (false, false);
    for (position, action) in &state.statements {
        if *position >= statement.start_byte() {
            break;
        }
        match action {
            Action::Zone => zone = true,
            Action::Environment => environment = true,
            Action::Barrier => {
                zone = false;
                environment = false;
            }
        }
    }
    zone && environment
}
fn integer(source: &SourceFile, node: Node<'_>) -> Option<i64> {
    integer_ast(source, node)?.to_string().parse().ok()
}
fn fixed_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if !bytes.is_ascii() || bytes.len() < 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let number = |start, end| {
        let digits = &value[start..end];
        digits
            .as_bytes()
            .iter()
            .all(u8::is_ascii_digit)
            .then(|| digits.parse::<u32>().ok())
            .flatten()
    };
    if !number(0, 4).is_some_and(|n| (1..=9999).contains(&n))
        || !number(5, 7).is_some_and(|n| (1..=12).contains(&n))
        || !number(8, 10).is_some_and(|n| (1..=28).contains(&n))
    {
        return false;
    }
    if bytes.len() == 10 {
        return true;
    }
    bytes.len() >= 19
        && matches!(bytes[10], b'T' | b' ')
        && bytes[13] == b':'
        && bytes[16] == b':'
        && number(11, 13).is_some_and(|n| n < 24)
        && number(14, 16).is_some_and(|n| n < 60)
        && number(17, 19).is_some_and(|n| n < 60)
        && matches!(&value[19..], "" | "Z" | "+00:00" | "+0000")
}
fn fixed_constructor(source: &SourceFile, node: Node<'_>, method: &[u8]) -> bool {
    let args = arguments(source, node);
    let Some(values) = args
        .iter()
        .map(|arg| arg.node())
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    match method {
        b"at" => {
            matches!(values.as_slice(),[value] if integer(source,*value).is_some_and(|n|(-2_147_483_648..=2_147_483_647).contains(&n)) || float_value(source,*value).is_some_and(|n|n.is_finite() && (-2_147_483_648.0..=2_147_483_647.0).contains(&n)))
        }
        b"new" | b"local" => {
            if values.is_empty() || values.len() > 6 {
                return false;
            }
            let limits = [(1, 9999), (1, 12), (1, 28), (0, 23), (0, 59), (0, 59)];
            values.iter().zip(limits).all(|(value, (min, max))| {
                integer(source, *value).is_some_and(|n| (min..=max).contains(&n))
            })
        }
        b"parse" => {
            matches!(values.as_slice(),[value] if plain_string(source,*value).is_some_and(|value|fixed_date(&value)))
        }
        _ => false,
    }
}
fn expression<'n>(source: &SourceFile, node: Node<'n>) -> Option<(Node<'n>, bool)> {
    let mut expression = node;
    let mut primitive = false;
    while let Some(parent) = send_parent(source, expression) {
        if regular_send_receiver(source, parent) != Some(Some(expression)) {
            break;
        }
        let name = regular_send_name(source, parent)?;
        if primitive
            || !ACCEPTED_METHODS.contains(&name.as_ref())
            || !arguments(source, parent).is_empty()
            || target(parent).child_by_field_name("block").is_some()
        {
            return None;
        }
        primitive = matches!(name.as_ref(), b"to_i" | b"to_f");
        expression = parent;
    }
    Some((expression, primitive))
}
fn validate(source: &SourceFile, mut edits: Edits) -> Option<Edits> {
    edits.sort_by_key(|(start, end, _)| (*start, *end));
    if edits.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        return None;
    }
    let mut bytes = source.as_bytes().to_vec();
    for (start, end, replacement) in edits.iter().rev() {
        if *start > *end || *end > bytes.len() {
            return None;
        }
        bytes.splice(*start..*end, replacement.bytes());
    }
    if bytes.iter().filter(|byte| **byte == b'\n').count()
        != source
            .as_bytes()
            .iter()
            .filter(|byte| **byte == b'\n')
            .count()
    {
        return None;
    }
    let proposed = SourceFile::from_bytes("time_zone.rb", bytes);
    if crate::parse::parse_ruby(&proposed)
        .ok()?
        .root_node()
        .has_error()
    {
        return None;
    }
    Some(edits)
}
pub(super) fn time(source: &SourceFile, node: Node<'_>, strict: bool) -> Option<Edits> {
    if target(node).child_by_field_name("block").is_some() {
        return None;
    }
    let method = regular_send_name(source, node)?;
    let (last, primitive) = expression(source, node)?;
    let statement = top_statement(source, last)?;
    if !initialized(source, node, statement) {
        return None;
    }
    let consumed = value_used(source, last);
    let fixed = fixed_constructor(source, node, &method);
    if consumed && (!primitive || !fixed) {
        return None;
    }
    if !fixed
        && !(matches!(method.as_ref(), b"now" | b"new")
            && arguments(source, node).is_empty()
            && !consumed)
    {
        return None;
    }
    let receiver = regular_send_receiver(source, node).flatten()?;
    if !time_constant(source, receiver) || !collect_comments(receiver).is_empty() {
        return None;
    }
    let receiver_text = std::str::from_utf8(node_bytes(source, receiver)).ok()?;
    let mut edits = vec![(
        receiver.start_byte(),
        receiver.end_byte(),
        if strict {
            "Time.zone".into()
        } else {
            format!("{receiver_text}.zone")
        },
    )];
    if method.as_ref() == b"new" {
        let (start, end) = selector(node)?;
        edits.push((
            start,
            end,
            if arguments(source, node).is_empty() {
                "now"
            } else {
                "local"
            }
            .into(),
        ));
    }
    let mut current = Some(node);
    while let Some(call) = current {
        if regular_send_name(source, call).as_deref() == Some(b"in_time_zone")
            && arguments(source, call).is_empty()
        {
            let (start, end) = selector(call)?;
            let dot = start.checked_sub(1)?;
            if source.as_bytes().get(dot) != Some(&b'.') {
                return None;
            }
            edits.push((dot, end, String::new()));
        }
        current = send_parent(source, call);
    }
    validate(source, edits)
}
pub(super) fn string(source: &SourceFile, node: Node<'_>) -> Option<Edits> {
    if regular_send_name(source, node).is_none()
        || !arguments(source, node).is_empty()
        || target(node).child_by_field_name("block").is_some()
        || !collect_comments(node).is_empty()
    {
        return None;
    }
    let receiver = regular_send_receiver(source, node).flatten()?;
    if file_literal(source, receiver)
        || !plain_string(source, receiver).is_some_and(|value| fixed_date(&value))
    {
        return None;
    }
    let (last, primitive) = expression(source, node)?;
    let statement = top_statement(source, last)?;
    if (value_used(source, last) && !primitive) || !initialized(source, node, statement) {
        return None;
    }
    let literal = std::str::from_utf8(node_bytes(source, receiver)).ok()?;
    validate(
        source,
        vec![(
            node.start_byte(),
            node.end_byte(),
            format!("Time.zone.parse({literal})"),
        )],
    )
}
