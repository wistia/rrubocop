# frozen_string_literal: true

require_relative "parity_audit/reference"

class RSpecParityReference < AuditReference
  def refresh_all(input, output)
    rows = JSON.parse(File.read(input)).map.with_index do |test, index|
      result = audit(test, index)
      %w[safe all].each do |mode|
        next unless result[mode]

        after = test.merge("source" => result[mode].fetch(:source))
        result[mode][:diagnostics] = diagnostics(investigation(after, source(after, index), false))
      end
      test.merge("reference" => result)
    end
    File.write(output, JSON.pretty_generate(rows) + "\n")
  end

  private

  def native_compile(_test)
    {}
  end
end

if $PROGRAM_NAME == __FILE__
  RSpecParityReference.new(ARGV.fetch(0), ARGV.fetch(1)).refresh_all(ARGV.fetch(1), ARGV.fetch(2))
end
