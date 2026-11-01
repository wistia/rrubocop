use super::*;

pub(super) fn record_block(
    open: &mut HashMap<String, DisableDirective>,
    out: &mut Vec<DisableDirective>,
    line: usize,
    column: usize,
    name: String,
) {
    let previous = open.remove(&open_block_key(open, name.clone()));
    if let Some(mut previous) = previous {
        previous.range.1 = line;
        out.push(previous);
    }
    open.insert(
        name.clone(),
        DisableDirective {
            line,
            column,
            cops: vec![name],
            range: (line, line),
        },
    );
}
