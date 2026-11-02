# frozen_string_literal: true

require 'bundler/setup'
require 'json'
require 'rubocop'

def signed_command?(node)
  argument = node.arguments.first
  argument&.source&.start_with?('+') && node.loc.begin.nil? && (node.loc.dot || !node.operator_method?)
end

def call_record(node)
  selector = node.loc.selector
  { selector: selector.source, source: node.source,
    position: [selector.line, selector.column], receiver: node.receiver&.source,
    arguments: node.arguments.map { |arg| { type: arg.type, source: arg.source } } }
end

def signed_calls(parsed)
  return [] unless parsed.valid_syntax?

  parsed.ast&.each_node(:send, :csend)&.select { |node| signed_command?(node) }&.map { |node| call_record(node) } || []
end

sources = JSON.parse(File.read(ARGV.fetch(0))).map { |row| row.is_a?(Hash) ? row.fetch('source') : row }.uniq
rows = sources.flat_map do |text|
  [2.5, 3.0, 3.4].map do |version|
    parsed = RuboCop::ProcessedSource.new(text, version)
    tokens = parsed.tokens.select { |token| token.text == '+' }.map do |token|
      { type: token.type, source: token.text, position: [token.pos.line, token.pos.column] }
    end
    { source: text, ruby_version: version, valid_syntax: parsed.valid_syntax?,
      calls: signed_calls(parsed), tokens: tokens }
  end
end
File.write(ARGV.fetch(1), "#{JSON.pretty_generate(rows)}\n")
