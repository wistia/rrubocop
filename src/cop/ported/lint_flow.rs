mod constant_or;
use constant_or::constant_or;
mod multiple_comparison;
use multiple_comparison::multiple_comparison;
mod regexp_condition;
use regexp_condition::regexp_condition;
mod rescue_type;
use rescue_type::rescue_type;
mod returns;
use returns::{top_return, void_return};
mod self_assignment;
use self_assignment::self_assignment;
mod safe_empty;
use safe_empty::safe_empty;
mod safe_consistency;
use safe_consistency::safe_consistency;
mod float_comparison;
use crate::cop::CopRegistry;
use float_comparison::float_comparison;
pub(super) fn register(r: &mut CopRegistry) {
    super::rules!(r;
        ("Lint/OrAssignmentToConstant", &["operator_assignment"], constant_or, true, true, false),
        ("Lint/MultipleComparison", &["binary"], multiple_comparison, true, true, false),
        ("Lint/RegexpAsCondition", &["regex"], regexp_condition, true, true, false),
        ("Lint/RescueType", &["rescue"], rescue_type, true, true, false),
        ("Lint/ReturnInVoidContext", &["return"], void_return, false, true, false),
        ("Lint/TopLevelReturnWithArgument", &["return"], top_return, true, true, false),
        ("Lint/SelfAssignment", &["assignment","operator_assignment","call"], self_assignment, false, true, false),
        ("Lint/SafeNavigationWithEmpty", &["if","unless","if_modifier","unless_modifier","conditional"], safe_empty, true, true, false),
        ("Lint/SafeNavigationConsistency", &["binary"], safe_consistency, true, true, false),
        ("Lint/FloatComparison", &["binary","call"], float_comparison, false, true, false)
    );
}
