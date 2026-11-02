# frozen_string_literal: true

module AuditEncodingMutations
  def self.expand(source, _ast)
    %w[UTF-8 ASCII-8BIT].map do |encoding|
      lines = source.lines
      index = lines.first&.start_with?('#!') ? 1 : 0
      lines.insert(index, "# encoding: #{encoding}\n")
      ["source-encoding:#{encoding}", lines.join]
    end
  end
end
