# frozen_string_literal: true

require "json"
require "tmpdir"
require "fileutils"
require "rubocop"
require "rubocop-rails"
require "active_support/core_ext/object/blank"

fixtures = [
  [nil, nil], [nil, "mysql2://example/database"], [nil, "trilogy://example/database"],
  [nil, "postgres://example/database"], [nil, "postgresql://example/database"], [nil, "sqlite://example"],
  ["development:\n  adapter: mysql2\n", "postgres://example/database"],
  ["development:\n  adapter: trilogy\n", nil],
  ["development:\n  adapter: postgis\n", nil],
  ["development:\n  primary:\n    adapter: postgresql\n", nil],
  ["development:\n  adapter: sqlite\n", "mysql2://example/database"],
  ["production:\n  adapter: mysql2\n", nil],
  ["development: []\n", "postgres://example/database"],
  ["[invalid", "trilogy://example/database"],
  ["defaults: &defaults\n  adapter: mysql2\ndevelopment:\n  primary:\n    database: example\n    <<: *defaults\n", nil]
]
probe = Class.new do
  include RuboCop::Cop::DatabaseTypeResolvable

  def cop_config
    {}
  end
end.new
original_url = ENV["DATABASE_URL"]
rows = fixtures.map do |yaml, url|
  Dir.mktmpdir("rrubocop-schema-db-") do |directory|
    Dir.chdir(directory) do
      FileUtils.mkdir_p("config")
      File.write("config/database.yml", yaml) if yaml
      ENV["DATABASE_URL"] = url
      {yaml: yaml, url: url, database: probe.database}
    end
  end
end
ENV["DATABASE_URL"] = original_url
File.write(ARGV.fetch(0), JSON.pretty_generate(rows) + "\n")
puts JSON.generate(observations: rows.size)
