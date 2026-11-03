# frozen_string_literal: true

require "bundler/setup"
require "rubocop"
require "parser/ruby34"
require "json"
abort "Expected RuboCop 1.65.1" unless RuboCop::Version::STRING == "1.65.1"
Parser::Builders::Default.emit_procarg0 = true
Parser::Builders::Default.emit_arg_inside_procarg0 = true

module BlockCommaBindings
  def self.parameter_shapes(source)
    buffer = Parser::Source::Buffer.new("proof.rb")
    buffer.source = source
    parser = Parser::Ruby34.new
    parser.diagnostics.all_errors_are_fatal = true
    parser.diagnostics.ignore_warnings = true
    pending = [parser.parse(buffer)].compact
    result = []
    until pending.empty?
      node = pending.pop
      if node.type == :block
        result << node.children[1].children.map(&:type)
      end
      pending.concat(node.children.grep(Parser::AST::Node).reverse)
    end
    result
  end
end

rows = JSON.parse(File.read(ARGV.fetch(0)))
proof = rows.filter_map do |entry|
  before = BlockCommaBindings.parameter_shapes(entry.fetch("original"))
  after = BlockCommaBindings.parameter_shapes(entry.fetch("ruby_corrected"))
  next if before == after

  {id: entry.fetch("id"), original: entry.fetch("original"), ruby_corrected: entry.fetch("ruby_corrected"),
   native_expected: entry.fetch("original"), before_parameter_types: before, after_parameter_types: after,
   parser_version: Gem.loaded_specs.fetch("parser").version.to_s,
   semantic_parser_compatibility_flags: {emit_procarg0: true, emit_arg_inside_procarg0: true}}
end
File.write(ARGV.fetch(1), "#{JSON.pretty_generate(proof)}\n")
