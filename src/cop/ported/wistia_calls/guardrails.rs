use super::super::context::*;
use tree_sitter::Node;
pub(super) fn csv(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) == "generate" && c.receiver(n) == "CSV" {
        vec![Finding::range(
            n.start_byte(),
            field(n, "arguments")
                .unwrap_or_else(|| method_node(n))
                .end_byte(),
            "Avoid using `CSV.generate`. Please use `CSV.safe_generate` instead to avoid excel macro attacks.",
        )]
    } else {
        vec![]
    }
}
pub(super) fn content_index(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if matches!(
        c.receiver(n).trim_start_matches("::"),
        "AlgoliaContentIndex" | "AlgoliaVPaaSContentIndex"
    ) {
        vec![Finding::new(
            n,
            "Do not call Content Indices directly in Algolia workers. Use the helpers in Algolia::Backfill::BaseContentByAccountWorker instead.",
        )]
    } else {
        vec![]
    }
}
pub(super) fn lantern(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !matches!(
        c.receiver(n).trim_start_matches("::"),
        "config_builder" | "@config_builder" | "builder" | "@builder"
    ) || !MUTATORS.contains(&c.method(n))
    {
        return vec![];
    }
    let p = c.source.path_str();
    if p.contains("/spec/")
        || p.starts_with("spec/")
        || p.ends_with("video_remix_lantern_agent/mutations.rb")
        || p.ends_with("video_remix_lantern_agent/config_builder.rb")
    {
        return vec![];
    }
    vec![Finding::new(
        n,
        format!(
            "Direct ConfigBuilder mutator call. Route writes through `config_builder.apply_mutation(:{}, **payload)` so the PreWriteValidators + PostWriteCleanup pipeline runs. See VideoRemixLanternAgent::Mutations for the dispatch.",
            c.method(n)
        ),
    )]
}
const MUTATORS: &[&str] = &[
    "append_element",
    "update_element",
    "remove_element",
    "remove_elements",
    "replace_element_with",
    "move_element",
    "copy_element",
    "set_music",
    "clear_music",
    "set_narration",
    "clear_narration",
    "set_target_duration",
    "clear_target_duration",
    "set_theme_tokens",
    "clear_theme_tokens",
    "set_aspect_ratio",
    "update_timing",
    "remove_timing",
    "update_marker",
    "remove_marker",
    "set_timings",
    "set_markers",
];
pub(super) fn foreign_keys(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) == "add_foreign_key" {
        vec![Finding::new(
            n,
            "Foreign keys are ignored via database config. Use a different way to enforce referential integrity.",
        )]
    } else {
        vec![]
    }
}
pub(super) fn sleep(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) == "sleep" {
        vec![Finding::new(
            n,
            "Avoid using sleep in production code. Prefer using async jobs instead. If already in an async job consider using Pollable concern.",
        )]
    } else {
        vec![]
    }
}
pub(super) fn timers(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) == "periodically" {
        vec![Finding::new(
            n,
            "Periodical Timers are not supported in AnyCable",
        )]
    } else {
        vec![]
    }
}
