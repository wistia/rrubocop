use super::*;

#[derive(Default)]
pub(super) struct Parents(HashMap<&'static str, HashMap<&'static str, Vec<usize>>>);
impl Parents {
    pub(super) fn register(&mut self, index: usize, cop: &dyn Cop, kind: &'static str) -> bool {
        let parents = cop.interested_parent_node_kinds(kind);
        if parents.is_empty() || kind == "identifier" {
            return false;
        }
        for parent in parents {
            self.0
                .entry(kind)
                .or_default()
                .entry(parent)
                .or_default()
                .push(index);
        }
        true
    }
    pub(super) fn merge(&mut self, kinds: &HashMap<&'static str, Vec<usize>>) {
        for (kind, parents) in &mut self.0 {
            if let Some(ungated) = kinds.get(kind) {
                for indices in parents.values_mut() {
                    indices.extend(ungated);
                    indices.sort_unstable();
                }
            }
        }
    }
    pub(super) fn get(&self, kind: &str, parent: Option<&str>) -> Option<&Vec<usize>> {
        self.0.get(kind)?.get(parent?)
    }
}

#[cfg(test)]
mod tests;
