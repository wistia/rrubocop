mod active_record_aliases;
mod active_record_override;
mod active_support_aliases;
mod application_controller;
mod application_job;
mod application_mailer;
mod application_record;
mod arel_star;
mod assert_not;
mod belongs_to;
mod blank;
mod bulk_change_table;
mod content_tag;
mod create_table_with_timestamps;
mod database;
mod date;
mod delegate;
mod delegate_allow_blank;
mod dynamic_find_by;
mod eager_evaluation_log_message;
mod enforce_superclass;
mod enum_hash;
mod enum_uniqueness;
mod environment_comparison;
mod exit;
mod file_path;
mod find_by;
mod find_each;
mod has_and_belongs_to_many;
mod has_many_or_has_one_dependent;
mod helper_instance_variable;
mod http_positional_arguments;
mod http_status;
mod ignored_skip_action_filter_option;
mod index_by;
mod index_method;
mod index_with;
mod inverse_of;
mod lexically_scoped_action_filter;
mod link_to_blank;
mod literal_ast_value;
mod literal_value;
mod migration_body;
mod not_null_column;
mod output;
mod output_safety;
mod pick;
mod pluralization_grammar;
mod presence;
mod present;
mod rake_environment;
mod read_write_attribute;
mod redundant_active_record_all_method;
mod redundant_allow_nil;
mod redundant_foreign_key;
mod redundant_receiver_in_with_options;
mod reflection_class_name;
mod refute_methods;
mod relative_date_constant;
mod request_referer;
mod reversible_migration;
mod safe_navigation;
mod safe_navigation_with_blank;
mod schema_model;
mod scope_args;
mod skips_model_validations;
mod time_zone;
mod uniq_before_pluck;
mod unique_validation_without_index;
mod unknown_env;
mod validation;

use crate::cop::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    crate::register_cops!(registry;
        active_record_aliases::ActiveRecordAliases,
        active_record_override::ActiveRecordOverride,
        active_support_aliases::ActiveSupportAliases,
        application_controller::ApplicationController,
        application_job::ApplicationJob,
        application_mailer::ApplicationMailer,
        application_record::ApplicationRecord,
        arel_star::ArelStar,
        assert_not::AssertNot,
        belongs_to::BelongsTo,
        blank::Blank,
        bulk_change_table::BulkChangeTable,
        content_tag::ContentTag,
        create_table_with_timestamps::CreateTableWithTimestamps,
        date::Date,
        delegate::Delegate,
        delegate_allow_blank::DelegateAllowBlank,
        dynamic_find_by::DynamicFindBy,
        eager_evaluation_log_message::EagerEvaluationLogMessage,
        enum_hash::EnumHash,
        enum_uniqueness::EnumUniqueness,
        environment_comparison::EnvironmentComparison,
        exit::Exit,
        file_path::FilePath,
        find_by::FindBy,
        find_each::FindEach,
        has_and_belongs_to_many::HasAndBelongsToMany,
        has_many_or_has_one_dependent::HasManyOrHasOneDependent,
        helper_instance_variable::HelperInstanceVariable,
        http_positional_arguments::HttpPositionalArguments,
        http_status::HttpStatus,
        ignored_skip_action_filter_option::IgnoredSkipActionFilterOption,
        index_by::IndexBy,
        index_with::IndexWith,
        inverse_of::InverseOf,
        lexically_scoped_action_filter::LexicallyScopedActionFilter,
        link_to_blank::LinkToBlank,
        not_null_column::NotNullColumn,
        output::Output,
        output_safety::OutputSafety,
        pick::Pick,
        pluralization_grammar::PluralizationGrammar,
        presence::Presence,
        present::Present,
        rake_environment::RakeEnvironment,
        read_write_attribute::ReadWriteAttribute,
        redundant_active_record_all_method::RedundantActiveRecordAllMethod,
        redundant_allow_nil::RedundantAllowNil,
        redundant_foreign_key::RedundantForeignKey,
        redundant_receiver_in_with_options::RedundantReceiverInWithOptions,
        reflection_class_name::ReflectionClassName,
        refute_methods::RefuteMethods,
        relative_date_constant::RelativeDateConstant,
        request_referer::RequestReferer,
        reversible_migration::ReversibleMigration,
        safe_navigation::SafeNavigation,
        safe_navigation_with_blank::SafeNavigationWithBlank,
        scope_args::ScopeArgs,
        skips_model_validations::SkipsModelValidations,
        time_zone::TimeZone,
        uniq_before_pluck::UniqBeforePluck,
        unique_validation_without_index::UniqueValidationWithoutIndex,
        unknown_env::UnknownEnv,
        validation::Validation,
    );
}

pub(crate) use database::from_directory as database_dependency_adapter;
pub(crate) use schema_model::path_from as schema_dependency_path;
