# frozen_string_literal: true

require 'json'

abort 'Expected reference Ruby 3.4.9' unless RUBY_VERSION == '3.4.9'

puts((Class.instance_methods + Class.private_instance_methods +
      Object.instance_methods + Object.private_instance_methods).map(&:to_s).uniq.sort)
