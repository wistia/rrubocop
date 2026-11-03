# frozen_string_literal: true

# Records literal argument grouping without executing authored programs.
module FrameworkArguments
  def self.argument(source, argument)
    range = argument.source_range
    {
      braces: argument.hash_type? && !argument.loc.begin.nil?,
      end: source[0...range.end_pos].bytesize,
      hash: argument.hash_type?,
      parts: argument.hash_type? ? argument.children.size : 1,
      start: source[0...range.begin_pos].bytesize
    }
  end

  def self.row(source)
    parsed = RuboCop::ProcessedSource.new(source, 3.4)
    raise "Invalid source: #{source}" if parsed.ast.nil? || parsed.parser_error

    target = parsed.ast.each_node(:send).find { |node| node.method_name == :probe }
    { arguments: target.arguments.map { |node| argument(source, node) }, source: }
  end

  def self.run
    gem 'rubocop', '1.65.1'
    require 'rubocop'
    require 'json'
    rows = JSON.parse(File.read(ARGV.fetch(0))).fetch('cases').map { |entry| row(entry.fetch('source')) }
    puts JSON.pretty_generate(cases: rows, rubocop: RuboCop::Version::STRING, source: 'SendNode#arguments')
  end
end

FrameworkArguments.run if $PROGRAM_NAME == __FILE__
