# frozen_string_literal: true

require 'bundler/setup'
require 'json'
require 'rubocop'
rows = JSON.parse(File.read(ARGV[0]))
rows.each do |row|
  ast = RuboCop::ProcessedSource.new(row.fetch('source'), row.fetch('ruby_version')).ast
  value = if ast&.regexp_type? && ast.children.size == 2 &&
             ast.children.last.children.empty? && ast.children.first&.str_type?
            ast.children.first.value.dup.force_encoding(Encoding::UTF_8)
          end
  row['value'] = value&.valid_encoding? && !value.empty? ? value : nil
end
File.write(ARGV[1], "#{JSON.pretty_generate(rows)}\n")
