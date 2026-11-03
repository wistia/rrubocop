use crate::cop::CopConfig;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Style {
    Key,
    Separator,
    Table,
    Splat,
}
impl Style {
    pub(super) fn message(self) -> &'static str {
        match self {
            Self::Key => "Align the keys of a hash literal if they span more than one line.",
            Self::Separator => {
                "Align the separators of a hash literal if they span more than one line."
            }
            Self::Table => {
                "Align the keys and values of a hash literal if they span more than one line."
            }
            Self::Splat => {
                "Align keyword splats with the rest of the hash if it spans more than one line."
            }
        }
    }
}
pub(super) fn styles(config: &CopConfig, rocket: bool) -> Vec<Style> {
    let key = if rocket {
        "EnforcedHashRocketStyle"
    } else {
        "EnforcedColonStyle"
    };
    let value = config
        .options
        .get(key)
        .or_else(|| config.options.get("EnforcedStyle"));
    let values: Vec<&str> = match value {
        Some(serde_yml::Value::Sequence(values)) => {
            values.iter().filter_map(|v| v.as_str()).collect()
        }
        Some(value) => value.as_str().into_iter().collect(),
        None => vec!["key"],
    };
    let mut result = Vec::new();
    for name in values {
        let style = match name {
            "key" => Style::Key,
            "separator" => Style::Separator,
            "table" => Style::Table,
            _ => continue,
        };
        if !result.contains(&style) {
            result.push(style);
        }
    }
    result
}
#[derive(Clone, Copy)]
pub(super) struct Element<'a> {
    pub(super) node: Node<'a>,
    pub(super) key: Node<'a>,
    pub(super) value: Option<Node<'a>>,
    pub(super) operator: Option<Node<'a>>,
    pub(super) rocket: bool,
}
impl<'a> Element<'a> {
    pub(super) fn new(node: Node<'a>) -> Option<Self> {
        if node.kind() == "hash_splat_argument" {
            return Some(Self {
                node,
                key: node,
                value: None,
                operator: None,
                rocket: false,
            });
        }
        if node.kind() != "pair" {
            return None;
        }
        let key = node.child_by_field_name("key")?;
        let mut cursor = node.walk();
        let operator = node
            .children(&mut cursor)
            .find(|n| matches!(n.kind(), ":" | "=>"))?;
        Some(Self {
            node,
            key,
            value: node.child_by_field_name("value"),
            operator: Some(operator),
            rocket: operator.kind() == "=>",
        })
    }
    pub(super) fn splat(self) -> bool {
        self.operator.is_none()
    }
    pub(super) fn omitted(self) -> bool {
        !self.splat() && self.value.is_none()
    }
}
pub(super) fn col(source: &SourceFile, node: Node<'_>) -> i64 {
    source.offset_to_line_col(node.start_byte()).1 as i64
}
pub(super) fn end_col(source: &SourceFile, node: Node<'_>) -> i64 {
    source.offset_to_line_col(node.end_byte()).1 as i64
}
pub(super) fn line(source: &SourceFile, node: Node<'_>) -> usize {
    source.offset_to_line_col(node.start_byte()).0
}
pub(super) fn last_line(source: &SourceFile, node: Node<'_>) -> usize {
    source.offset_to_line_col(node.end_byte()).0
}
pub(super) fn same_line(source: &SourceFile, first: Element<'_>, current: Element<'_>) -> bool {
    last_line(source, first.node) == line(source, current.node)
        || line(source, first.node) == last_line(source, current.node)
}
pub(super) fn begins_line(source: &SourceFile, node: Node<'_>) -> bool {
    let start = source.line_start(line(source, node)).unwrap_or(0);
    source.as_bytes()[start..node.start_byte()]
        .iter()
        .all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\x0b' | b'\x0c'))
}
#[derive(Clone, Copy, Default)]
pub(super) struct Delta {
    pub(super) key: i64,
    pub(super) separator: i64,
    pub(super) value: i64,
}
impl Delta {
    pub(super) fn bad(self) -> bool {
        self.key != 0 || self.separator != 0 || self.value != 0
    }
}
pub(super) fn key_delta(
    source: &SourceFile,
    first: Element<'_>,
    current: Element<'_>,
    right: bool,
) -> i64 {
    if same_line(source, first, current) || (right && (first.splat() || current.splat())) {
        0
    } else if right {
        end_col(source, first.key) - end_col(source, current.key)
    } else {
        col(source, first.key) - col(source, current.key)
    }
}
pub(super) fn deltas(
    source: &SourceFile,
    first: Element<'_>,
    current: Element<'_>,
    style: Style,
    width: i64,
    initial: bool,
) -> Delta {
    if style == Style::Splat {
        return Delta {
            key: if begins_line(source, current.node) {
                key_delta(source, first, current, false)
            } else {
                0
            },
            ..Delta::default()
        };
    }
    if style == Style::Separator && initial {
        return Delta::default();
    }
    if style == Style::Key && !initial && !begins_line(source, current.node) {
        return Delta::default();
    }
    let op = current.operator.unwrap();
    let value = current.value.unwrap_or(current.key);
    let key = if initial {
        0
    } else {
        key_delta(source, first, current, style == Style::Separator)
    };
    let separator = if !current.rocket {
        0
    } else {
        match style {
            Style::Key => end_col(source, current.key) + 1 - col(source, op),
            Style::Table => col(source, first.key) + width + 1 - col(source, op) - key,
            Style::Separator => {
                let raw = if same_line(source, first, current) || first.rocket != current.rocket {
                    0
                } else {
                    col(source, first.operator.unwrap()) - col(source, op)
                };
                raw - key
            }
            Style::Splat => unreachable!(),
        }
    };
    let raw_value = match style {
        Style::Key => {
            if current.omitted() || line(source, current.key) != line(source, value) {
                0
            } else {
                end_col(source, op) + 1 - col(source, value)
            }
        }
        Style::Table => {
            if current.omitted() {
                0
            } else {
                col(source, first.key) + if current.rocket { 4 } else { 2 } + width
                    - col(source, value)
            }
        }
        Style::Separator => {
            if current.omitted() || same_line(source, first, current) {
                0
            } else {
                col(source, first.value.unwrap_or(first.key)) - col(source, value)
            }
        }
        Style::Splat => unreachable!(),
    };
    Delta {
        key,
        separator,
        value: if style == Style::Key {
            raw_value
        } else {
            raw_value - key - separator
        },
    }
}
