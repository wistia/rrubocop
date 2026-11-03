# frozen_string_literal: true

# Captures the default inflections used by the pinned Rails cops.
module FrameworkInflections
  def self.names
    ['Post', 'Admin::Post', '::Admin::HTTPServer', 'HTMLTidy', 'HTTPServer', 'IPv6Address', 'X509Certificate',
     'FooBARBaz', 'foo-Bar', '-Foo-', 'Object', '', '::', 'foo::', '#<Class:Post>', '#<Class:Admin::Post>',
     'Éléphant', 'ÉléphantFoo', 'ΣValue', 'ΟΣ', 'AΟΣ', 'ΟΣA', 'İValue', '١Foo', '９Foo', 'Foo١Bar', 'Foo９Bar',
     'é', 'Foo__Bar'] + (0..127).flat_map { |byte| [byte.chr, "Foo#{byte.chr}Bar"] }
  end

  def self.capture
    raise 'Unexpected acronym customization' unless ActiveSupport::Inflector.inflections.acronyms.empty?

    {
      active_support: Gem.loaded_specs.fetch('activesupport').version.to_s,
      cases: names.map { |name| { foreign_key: name.foreign_key, source: name, underscore: name.underscore } },
      source: 'active_support/inflector/methods.rb underscore/foreign_key with default empty acronyms'
    }
  end

  def self.run
    require 'json'
    gem 'activesupport', '8.1.3.1'
    require 'active_support/inflector'
    puts JSON.pretty_generate(capture)
  end
end

FrameworkInflections.run if $PROGRAM_NAME == __FILE__
