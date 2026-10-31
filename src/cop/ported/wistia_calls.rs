mod migrations;
use migrations::{charset, collation};
mod protected_writes;
use protected_writes::protected_writes;
mod sidekiq_receive;
use sidekiq_receive::sidekiq_receive;
mod guardrails;
use crate::cop::CopRegistry;
use guardrails::{content_index, csv, foreign_keys, lantern, sleep, timers};
const CALLS: &[&str] = &["call"];
pub(super) fn register(r: &mut CopRegistry) {
    super::rules!(r;
        ("Cops/CreateTableMustIncludeCharset", CALLS, charset, true, true, true),
        ("Cops/CreateTableMustIncludeCollation", CALLS, collation, true, true, true),
        ("Cops/NoCSVGenerate", CALLS, csv, false, true, true),
        ("Cops/NoCallbackSkippingProtectedWrites", CALLS, protected_writes, false, true, true),
        ("Cops/NoContentIndexCallsInBackfills", CALLS, content_index, false, true, true),
        ("Cops/NoDirectLanternConfigBuilderMutators", CALLS, lantern, false, true, true),
        ("Cops/NoForeignKeys", CALLS, foreign_keys, false, true, true),
        ("Cops/NoSleep", CALLS, sleep, false, true, true),
        ("Cops/NoSidekiqReceive", CALLS, sidekiq_receive, false, true, true),
        ("AnyCable/PeriodicalTimers", CALLS, timers, false, true, true)
    );
}
