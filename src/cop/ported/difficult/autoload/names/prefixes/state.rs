use std::sync::Arc;

#[derive(Clone, Default)]
pub(in crate::cop::ported::difficult::autoload::names) struct Namespace(Option<Arc<Segment>>);
struct Segment {
    parent: Namespace,
    name: String,
    head: String,
}
impl Namespace {
    pub(in crate::cop::ported::difficult::autoload::names) fn head(&self) -> &str {
        self.0.as_ref().map_or("Object", |s| &s.head)
    }
    pub(in crate::cop::ported::difficult::autoload::names) fn append(
        &self,
        name: String,
        limit: usize,
    ) -> Self {
        let head = super::super::head::join(self.0.as_ref().map(|s| s.head.as_str()), &name, limit);
        Self(Some(Arc::new(Segment {
            parent: self.clone(),
            name,
            head,
        })))
    }
    pub(in crate::cop::ported::difficult::autoload::names) fn full_name(&self) -> String {
        let mut names = Vec::new();
        let mut current = self;
        while let Some(segment) = &current.0 {
            names.push(segment.name.as_str());
            current = &segment.parent;
        }
        if names.is_empty() {
            return "Object".into();
        }
        names.reverse();
        names.join("::")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deeply_nested_prefixes_keep_a_bounded_head_and_exact_full_name() {
        let mut current = Namespace::default();
        for i in 0..1000 {
            current = current.append(format!("Module{i}"), 12);
            assert!(current.head().len() <= 15);
        }
        assert_eq!(current.head(), "Module0::Mod");
        assert!(current.full_name().ends_with("Module998::Module999"));
    }

    #[test]
    fn child_namespaces_share_their_parent_without_copying_its_parts() {
        let parent = Namespace::default().append("Outer".into(), 20);
        let child = parent.append("Inner".into(), 20);
        assert!(Arc::ptr_eq(
            parent.0.as_ref().unwrap(),
            child.0.as_ref().unwrap().parent.0.as_ref().unwrap()
        ));
        assert_eq!(child.full_name(), "Outer::Inner");
    }
}
