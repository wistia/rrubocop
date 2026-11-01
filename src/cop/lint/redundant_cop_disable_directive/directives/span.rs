use super::*;

pub(in crate::cop::lint::redundant_cop_disable_directive) fn offense_span(
    line: &str,
    dir: &DisableDirective,
    name: &str,
) -> (usize, usize) {
    if disable_marker(line).is_some_and(|(_, rest)| cop_names(rest).len() == 1)
        && let Some((_, end)) = nth_cop_token(line, name, 1)
    {
        return (
            dir.column,
            byte_index_to_column(line, end).saturating_sub(dir.column),
        );
    }
    let column = redundant_col(line, name, dir.column);
    (column, cop_highlight(line, column, name))
}
