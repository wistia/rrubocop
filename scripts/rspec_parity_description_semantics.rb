# frozen_string_literal: true

require "json"

original_events = []
native_events = []
upstream_events = []
original_value = proc { |value|
  original_events << value
  value
}
native_value = proc { |value|
  native_events << value
  value
}
upstream_value = proc { |value|
  upstream_events << value
  value
}

original_wording = "should do #{original_value.call("two  words")}"
native_wording = "should do #{native_value.call("two  words")}"
upstream_wording = "does #{upstream_value.call("two words")}"
raise "Native changed interpolation evaluation" unless original_events == native_events
raise "Original rewrite hazard missing" if original_events == upstream_events

safe_events = []
safe_value = proc { |value|
  safe_events << value
  value
}
original_spacing = "name #{safe_value.call(1)}"
corrected_spacing = "name #{safe_value.call(1)}"
raise "Safe expression whitespace changed a value" unless original_spacing == corrected_spacing && safe_events == [1, 1]

start_line = __LINE__ + 1
original_line = "should do " \
  "#{__LINE__ - start_line}"
start_line = __LINE__ + 1
upstream_line = "does #{__LINE__ - start_line}"
raise "Line-location hazard missing" unless original_line.end_with?("1") && upstream_line.end_with?("0")

puts JSON.pretty_generate({original_wording:, native_wording:, upstream_wording:,
                          original_events:, native_events:, upstream_events:,
                          original_spacing:, corrected_spacing:, safe_events:,
                          relative_line_payload: {original: 1, native: 1, upstream_rewrite: 0}})
