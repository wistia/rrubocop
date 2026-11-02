# frozen_string_literal: true

require 'tempfile'
require 'base64'

module AuditCapture
  # rubocop:disable Style/GlobalStdStream -- capture descriptor writes as well as Ruby global streams
  def self.run(&block)
    Tempfile.create('audit-out') do |out|
      Tempfile.create('audit-err') { |err| redirect(out, err, &block) }
    end
  end

  def self.redirect(out, err)
    saved_out = STDOUT.dup
    saved_err = STDERR.dup
    STDOUT.reopen(out)
    STDERR.reopen(err)
    $stdout = STDOUT
    $stderr = STDERR
    result = yield
    STDOUT.flush
    STDERR.flush
    result.merge(stdout: contents(out), stderr: contents(err))
  ensure
    STDOUT.reopen(saved_out)
    STDERR.reopen(saved_err)
    saved_out.close
    saved_err.close
    $stdout = STDOUT
    $stderr = STDERR
  end

  def self.contents(file)
    file.rewind
    Base64.strict_encode64(file.read)
  end
  # rubocop:enable Style/GlobalStdStream
end
