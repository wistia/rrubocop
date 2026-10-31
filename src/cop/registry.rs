use std::collections::HashMap;

use super::Cop;

/// Register many cops without inflating ABC (macro body is not scored as branches).
#[macro_export]
macro_rules! register_cops {
    ($registry:expr; $($cop:expr),+ $(,)?) => {{
        $($registry.register(::std::boxed::Box::new($cop));)+
    }};
}

pub struct CopRegistry {
    cops: Vec<Box<dyn Cop>>,
    index: HashMap<&'static str, usize>,
}

impl CopRegistry {
    pub fn new() -> Self {
        Self {
            cops: Vec::new(),
            index: HashMap::new(),
        }
    }

    pub fn default_registry() -> Self {
        let mut registry = Self::new();
        super::bundler::register_all(&mut registry);
        super::factory_bot::register_all(&mut registry);
        super::gemspec::register_all(&mut registry);
        super::graphql::register_all(&mut registry);
        super::layout::register_all(&mut registry);
        super::lint::register_all(&mut registry);
        super::metrics::register_all(&mut registry);
        super::naming::register_all(&mut registry);
        super::performance::register_all(&mut registry);
        super::rails::register_all(&mut registry);
        super::rake::register_all(&mut registry);
        super::rspec::register_all(&mut registry);
        super::rspec_rails::register_all(&mut registry);
        super::security::register_all(&mut registry);
        register_style_and_extensions(&mut registry);
        registry
    }

    pub fn register(&mut self, cop: Box<dyn Cop>) {
        let name = cop.name();
        let idx = self.cops.len();
        self.cops.push(cop);
        self.index.insert(name, idx);
    }

    pub fn cops(&self) -> &[Box<dyn Cop>] {
        &self.cops
    }

    pub fn get(&self, name: &str) -> Option<&dyn Cop> {
        self.index.get(name).map(|&idx| &*self.cops[idx])
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.cops.iter().map(|c| c.name()).collect()
    }

    pub fn len(&self) -> usize {
        self.cops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cops.is_empty()
    }

    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.index.get(name).copied()
    }
}

fn register_style_and_extensions(registry: &mut CopRegistry) {
    super::style::register_all(registry);
    super::ported::register_all(registry);
}
