mod policies;
use policies::{no_show, policy_scope};
mod queue;
use queue::queue;
mod constants;
use constants::spec_constants;
mod autoload;
use autoload::top_level;
mod migration;
use crate::cop::CopRegistry;
use migration::migration;
pub(super) fn register(r: &mut CopRegistry) {
    super::rules!(r;
        ("Cops/EnforceNoShowInPolicies", &["method"], no_show, false, true, true),
        ("Cops/EnforceScopeInPolicies", &["class"], policy_scope, false, true, true),
        ("Cops/ExplicitSidekiqQueue", &["class"], queue, false, true, true),
        ("Cops/NoConstantDefinitionsInSpecs", &["class","module","assignment","operator_assignment","for"], spec_constants, false, true, true),
        ("Cops/NoTopLevelSideEffectsInAutoloadedFiles", &["program"], top_level, false, true, true),
        ("Rails/ReversibleMigrationMethodDefinition", &["class"], migration, false, true, false)
    );
}

#[cfg(test)]
mod parity_tests;

#[cfg(test)]
mod shape_tests;
