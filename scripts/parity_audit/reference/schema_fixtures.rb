# frozen_string_literal: true

module SchemaFixtures
  def configure_schema_fixtures(cop, test)
    options = test.fetch("config", {})
    if options.key?("SchemaSource")
      source = options["SchemaSource"]
      ast = RuboCop::ProcessedSource.new(source, cop.target_ruby_version).ast if source
      schema = RuboCop::Rails::SchemaLoader::Schema.new(ast) if ast
      cop.define_singleton_method(:schema) { schema }
    end
    if options.key?("SchemaDatabaseYAML")
      yaml = options["SchemaDatabaseYAML"]
      development = yaml["development"] if yaml.is_a?(Hash)
      development = nil unless development.is_a?(Hash)
      cop.define_singleton_method(:database_yaml) { development }
    end
    return unless options.key?("SchemaDatabaseURL")

    original = cop.method(:database_from_env)
    url = options["SchemaDatabaseURL"]
    cop.define_singleton_method(:database_from_env) do
      previous = ENV["DATABASE_URL"]
      begin
        ENV["DATABASE_URL"] = url
        original.call
      ensure
        ENV["DATABASE_URL"] = previous
      end
    end
  end
end
