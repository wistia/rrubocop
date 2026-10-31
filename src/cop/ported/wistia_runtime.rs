mod anycable;
use anycable::{instance_vars, stream};
mod autoload;
use autoload::require_autoload;
mod redis;
use redis::redis_expiry;
mod rescue;
use rescue::report_error;
mod respond_to;
use respond_to::respond_to;
mod serializers;
use crate::cop::CopRegistry;
use serializers::{capitalized, key_block, method_block};
pub(super) fn register(r: &mut CopRegistry) {
    super::rules!(r;
        ("AnyCable/InstanceVars", &["instance_variable"], instance_vars, false, true, true),
        ("AnyCable/StreamFrom", &["call"], stream, false, true, true),
        ("Cops/NoRequireOnAutoLoadedPaths", &["call"], require_autoload, false, true, true),
        ("Cops/NoSetWithoutExpiration", &["call"], redis_expiry, false, true, true),
        ("Cops/RescueMustReportError", &["rescue","rescue_modifier"], report_error, false, true, true),
        ("Custom/RequireRespondToAny", &["call"], respond_to, false, true, true),
        ("Serializers/AvoidCapitalizedKeys", &["call"], capitalized, false, true, true),
        ("Serializers/AvoidKeyAndBlock", &["call"], key_block, false, true, true),
        ("Serializers/AvoidMethodAndBlock", &["call"], method_block, false, true, true)
    );
}
