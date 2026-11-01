use super::*;

pub(super) struct Switch {
    pub name: String,
    pub line: usize,
    pub disabled: bool,
    pub single_line: bool,
}

impl Directives {
    pub(crate) fn restarts_block(&self, cop: &str, line: usize) -> bool {
        self.switches.iter().any(|switch| {
            switch.line == line
                && switch.disabled
                && !switch.single_line
                && matches_cop(&switch.name, cop)
        }) && self
            .cop_ranges(cop)
            .iter()
            .any(|&(start, end)| start < line && line <= end)
    }
    pub(super) fn cop_ranges(&self, cop: &str) -> Arc<Vec<(usize, usize)>> {
        if self.switches.is_empty() {
            return Arc::default();
        }
        let mut ranges = self.ranges.lock().unwrap();
        ranges
            .entry(cop.to_ascii_lowercase())
            .or_insert_with(|| Arc::new(self.build_ranges(cop)))
            .clone()
    }

    fn build_ranges(&self, cop: &str) -> Vec<(usize, usize)> {
        let mut ranges = Vec::new();
        let mut start = None;
        for switch in self.switches.iter().filter(|s| matches_cop(&s.name, cop)) {
            if switch.single_line {
                if switch.disabled {
                    ranges.push((switch.line, switch.line));
                }
                continue;
            }
            if let Some(previous) = start.take() {
                ranges.push((previous, switch.line));
            }
            if switch.disabled {
                start = Some(switch.line);
            }
        }
        if let Some(start) = start {
            ranges.push((start, usize::MAX));
        }
        ranges
    }
}

fn matches_cop(name: &str, cop: &str) -> bool {
    if cop.eq_ignore_ascii_case("Lint/Syntax") {
        return false;
    }
    if cop.eq_ignore_ascii_case("Lint/RedundantCopDisableDirective") {
        return name.eq_ignore_ascii_case(cop);
    }
    name.eq_ignore_ascii_case(cop)
        || name.eq_ignore_ascii_case("all")
        || cop
            .split_once('/')
            .is_some_and(|(department, _)| name.eq_ignore_ascii_case(department))
}
