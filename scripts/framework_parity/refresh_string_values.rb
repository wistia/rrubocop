# frozen_string_literal: true

# Records AST values without executing authored fixture programs.
module FrameworkStringValues
  def self.row(source)
    parsed = RuboCop::ProcessedSource.new(source, 3.4)
    raise "Invalid source: #{source}" if parsed.ast.nil? || parsed.parser_error

    target = parsed.ast.each_node(:send).find { |node| node.method_name == :probe }.first_argument
    { ast_type: target.type, source:, value: target.value }
  end

  def self.run
    gem 'rubocop', '1.65.1'
    require 'rubocop'
    require 'json'
    rows = JSON.parse(File.read(ARGV.fetch(0))).fetch('cases').map { |entry| row(entry.fetch('source')) }
    puts JSON.pretty_generate(cases: rows, rubocop: RuboCop::Version::STRING, source: 'StrNode/DstrNode#value')
  end
end

FrameworkStringValues.run if $PROGRAM_NAME == __FILE__
