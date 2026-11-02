# frozen_string_literal: true

require 'json'
require 'tmpdir'
require 'base64'
require_relative 'runtime/capture'

class AuditRuntime
  def run(input)
    Dir.mktmpdir('rrubocop-runtime-') do |directory|
      Dir.chdir(directory) { scenario(input) }
    end
  end

  def scenario(input)
    input.fetch('files', {}).each { |name, contents| File.binwrite(relative(name), contents) }
    execution = execution_mode(input)
    result = AuditCapture.run { evaluate(input.fetch('source'), execution) }
    result[:files] = input.fetch('watch', []).to_h { |name| [name, snapshot(relative(name))] }
    result
  end

  def execution_mode(input)
    execution = input.fetch('execution', 'eval')
    raise ArgumentError, 'unknown runtime execution mode' unless %w[eval file].include?(execution)

    execution
  end

  def evaluate(source, execution)
    case execution
    when 'eval'
      eval(source, TOPLEVEL_BINDING, 'probe.rb') # rubocop:disable Security/Eval
    when 'file'
      File.write('probe.rb', source)
      load './probe.rb'
    else
      raise ArgumentError, 'unknown runtime execution mode'
    end
    {}
  rescue Exception => e # rubocop:disable Lint/RescueException
    exception = { class: e.class.name, message: e.message.lines.first.to_s.strip }
    exception[:status] = e.status if e.is_a?(SystemExit)
    { exception: }
  end

  def relative(name)
    raise ArgumentError, 'runtime paths must be simple filenames' unless simple_filename?(name)

    name
  end

  def simple_filename?(name)
    !name.empty? && File.basename(name) == name && !%w[. ..].include?(name)
  end

  def snapshot(name)
    File.file?(name) ? Base64.strict_encode64(File.binread(name)) : nil
  end
end

$stdout.puts(JSON.generate(AuditRuntime.new.run(JSON.parse($stdin.read)))) if $PROGRAM_NAME == __FILE__
