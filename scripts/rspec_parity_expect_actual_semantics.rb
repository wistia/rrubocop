# frozen_string_literal: true

require "json"
require "rspec/expectations"

class AsymmetricExpectationValue
  def ==(other)
    other == :literal
  end

  def eql?(other)
    other == :literal
  end

  def equal?(other)
    other == :literal
  end
end

class ExpectActualSemanticProbe
  include RSpec::Matchers

  def compare(method)
    object = AsymmetricExpectationValue.new
    original = public_send(method, object).matches?(:literal)
    upstream_rewrite = public_send(method, :literal).matches?(object)
    raise "Asymmetric comparison hazard missing" unless original == false && upstream_rewrite == true
    {original:, upstream_rewrite:}
  end

  def scalar
    original = eq(2).matches?(1)
    corrected = eq(1).matches?(2)
    raise "Immutable scalar comparison changed" unless original == corrected && !original
    {original:, corrected:}
  end
end

probe = ExpectActualSemanticProbe.new
comparisons = {}
%i[eq eql equal be].each { |method| comparisons.store(method, probe.compare(method)) }
puts JSON.pretty_generate({comparisons:, scalar: probe.scalar})
