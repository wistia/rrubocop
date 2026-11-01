use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, CopRegistry};
use std::path::Path;
use std::sync::OnceLock;

pub(crate) struct PreparedConfigs {
    root: Group,
    layers: Vec<OnceLock<Group>>,
    syntax: OnceLock<CopConfig>,
}

struct Group {
    configs: Vec<OnceLock<CopConfig>>,
    effective: OnceLock<ResolvedConfig>,
    disabled: Vec<bool>,
}

#[derive(Clone, Copy)]
pub(crate) struct PreparedLint<'a> {
    pub config: &'a ResolvedConfig,
    pub registry: &'a CopRegistry,
    pub filters: &'a CopFilterSet,
    pub configs: &'a PreparedConfigs,
}

impl PreparedConfigs {
    pub(crate) fn new(config: &ResolvedConfig, registry: &CopRegistry) -> Self {
        Self {
            root: Group::new(registry, Vec::new()),
            layers: (0..config.dir_overrides.len())
                .map(|_| OnceLock::new())
                .collect(),
            syntax: OnceLock::new(),
        }
    }

    fn group(&self, context: PreparedLint<'_>, path: &Path) -> (&Group, bool) {
        let Some(index) = context.config.dir_layer_index_for_file(path) else {
            return (&self.root, false);
        };
        let group = self.layers[index].get_or_init(|| {
            let disabled = context
                .registry
                .cops()
                .iter()
                .map(|cop| context.config.dir_override_disables(index, cop.name()))
                .collect();
            Group::new(context.registry, disabled)
        });
        (group, true)
    }
}

impl Group {
    fn new(registry: &CopRegistry, disabled: Vec<bool>) -> Self {
        Self {
            configs: registry.cops().iter().map(|_| OnceLock::new()).collect(),
            effective: OnceLock::new(),
            disabled,
        }
    }

    fn config(&self, context: PreparedLint<'_>, path: &Path) -> &ResolvedConfig {
        self.effective.get_or_init(|| {
            context
                .config
                .effective_config_for_file(path)
                .expect("directory group has a layer")
        })
    }
}

pub(crate) struct FileConfigs<'a> {
    context: PreparedLint<'a>,
    group: &'a Group,
    path: &'a Path,
    nested: bool,
}

impl<'a> PreparedLint<'a> {
    pub(crate) fn for_file(self, path: &'a Path) -> FileConfigs<'a> {
        let (group, nested) = self.configs.group(self, path);
        FileConfigs {
            context: self,
            group,
            path,
            nested,
        }
    }

    pub(crate) fn root_config(self, index: usize) -> &'a CopConfig {
        self.configs.root.configs[index]
            .get_or_init(|| self.config.cop_config(self.registry.cops()[index].name()))
    }

    pub(crate) fn syntax(self) -> &'a CopConfig {
        if let Some(index) = self.registry.index_of("Lint/Syntax") {
            self.root_config(index)
        } else {
            self.configs
                .syntax
                .get_or_init(|| self.config.cop_config("Lint/Syntax"))
        }
    }
}

impl<'a> FileConfigs<'a> {
    pub(crate) fn disabled(&self, index: usize) -> bool {
        self.group.disabled.get(index).copied().unwrap_or(false)
    }

    pub(crate) fn get(&self, index: usize) -> &'a CopConfig {
        self.group.configs[index].get_or_init(|| {
            let config = if self.nested {
                self.group.config(self.context, self.path)
            } else {
                self.context.config
            };
            config.cop_config(self.context.registry.cops()[index].name())
        })
    }
}

#[cfg(test)]
mod tests;
