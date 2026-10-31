use crate::cop::CopRegistry;
mod departments;
mod keywords;
mod methods;
mod mixins;
mod numbered;
mod serializers;
mod simple;
mod times;
pub(super) fn register(r: &mut CopRegistry) {
    super::rules!(r;
        ("Cops/NoSetex", &["call"], simple::setex, true, true, true),
        ("Lint/RefinementImportMethods", &["call"], simple::refinement, false, true, false),
        ("Lint/UselessTimes", &["call"], times::check, true, false, false),
        ("Lint/NumberedParameterAssignment", &["identifier"], numbered::check, false, true, false),
        ("Migration/DepartmentName", &["comment"], departments::check, true, true, false),
        ("Serializers/PreferAttributeBlockOverAttributeAndMethod", &["call"], serializers::single, true, true, true),
        ("Serializers/PreferAttributeBlockOverAttributesAndMethods", &["call"], serializers::plural, true, true, true),
        ("Style/ColonMethodDefinition", &["singleton_method"], simple::colon, true, true, false),
        ("Style/EmptyBlockParameter", &["block","do_block"], simple::empty_parameters, true, true, false),
        ("Style/KeywordParametersOrder", &["method_parameters","block_parameters","lambda_parameters"], keywords::check, true, true, false),
        ("Style/MixinGrouping", &["class","module"], mixins::check, true, true, false),
        ("Style/MultilineWhenThen", &["when"], simple::when_then, true, true, false),
        ("Style/TrailingBodyOnMethodDefinition", &["method","singleton_method"], methods::body, true, true, false),
        ("Style/TrailingMethodEndStatement", &["method"], methods::end, true, true, false)
    );
}

fn arguments(n: tree_sitter::Node<'_>) -> Vec<tree_sitter::Node<'_>> {
    super::context::args(n)
        .into_iter()
        .filter(|a| a.kind() != "comment")
        .collect()
}
