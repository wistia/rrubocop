# frozen_string_literal: true

require_relative '../parity_audit/reference'

reference = AuditReference.new(ARGV.fetch(0))
rows = JSON.parse(File.read(ARGV.fetch(1)))
rows.each_with_index do |row, index|
  source = reference.send(:source, row.merge('path' => 'app/models/framework_parity_probe.rb'), index)
  left, right = source.ast.children
  row['same'] = left == right
  row['eql'] = left.eql?(right)
  row['grouped'] = [left, right].uniq.length == 1
end
File.write(ARGV.fetch(2), "#{JSON.pretty_generate(rows)}\n")
