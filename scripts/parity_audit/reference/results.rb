# frozen_string_literal: true

module AuditResults
  def diagnostics(report)
    report.offenses.filter_map do |offense|
      next if offense.disabled?

      { line: offense.line, column: offense.column,
        message: utf8(offense.message.delete_prefix("#{offense.cop_name}: ")),
        correctable: offense.correctable?, length: [offense.column_length, 1].max,
        severity: offense.severity.to_s }
    end
  end

  def utf8(text)
    text.dup.force_encoding(Encoding::UTF_8).scrub
  end

  def compile(source)
    RubyVM::InstructionSequence.compile(source, 'probe.rb')
    { valid: true }
  rescue SyntaxError => e
    { valid: false, error: utf8(e.message) }
  end

  def corrected(source)
    { source:, compile: compile(source) }
  end
end
