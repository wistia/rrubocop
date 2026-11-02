use super::*;

pub(super) fn static_key(c: &Context<'_>, key: Node<'_>) -> bool {
    if !collection_use(c, key) {
        return atom(c, key);
    }
    static_collection(c, key)
}

fn static_collection(c: &Context<'_>, key: Node<'_>) -> bool {
    match key.kind() {
        "array" => kids(key).into_iter().all(|key| atom(c, key)),
        "symbol_array" => kids(key).into_iter().all(|word| symbol_word(c.text(word))),
        "string_array" => kids(key).into_iter().all(|word| string_word(c.text(word))),
        _ => false,
    }
}

fn collection_use(c: &Context<'_>, key: Node<'_>) -> bool {
    let Some(parent) = key.parent() else {
        return false;
    };
    if is_call(parent) {
        return matches!(c.method(parent), "include?" | "exclude?");
    }
    parent.kind() == "argument_list" && parent.parent().is_some_and(|call| c.method(call) == "in?")
}

fn atom(c: &Context<'_>, key: Node<'_>) -> bool {
    match key.kind() {
        "integer" | "float" | "nil" | "true" | "false" => true,
        "simple_symbol" | "delimited_symbol" => c.static_symbol(key).is_some(),
        "string" | "character" => c.ruby_static_string(key).is_some(),
        _ => false,
    }
}

fn symbol_word(word: &str) -> bool {
    let mut bytes = word.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn string_word(word: &str) -> bool {
    !word.is_empty() && !word.contains(['\\', '\'', '#', '\n', '\r'])
}
