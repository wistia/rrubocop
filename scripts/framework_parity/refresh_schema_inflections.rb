# frozen_string_literal: true

require "json"
require "active_support/inflector"

words = %w[axis testis octopus virus alias status bus buffalo tomato datum medium data media analysis basis crisis wife leaf hive baby city toy box match class dish matrix vertex index mouse louse mice lice ox oxen quiz person people man men child children sex move zombie equipment information rice money species series fish sheep jeans police User Account BlogPost RawScaledScorer HTMLAPIUser admin_person AdminFish ΩΣ ΓΣΟΣ Key Éclair Niño]
names = [""] + words.flat_map { |word| [word, word.upcase, word.capitalize, "Admin_#{word}", "Namespace::#{word}"] }
names += ["fish\n", "fish\r\n", "person\nvalue", "value\nperson", "foo-fish", "foo fish", "namespace/Person"]
rows = names.uniq.map { |source| {source: source, pluralize: source.pluralize, tableize: source.tableize} }
File.write(ARGV.fetch(0), JSON.pretty_generate(active_support: Gem.loaded_specs.fetch("activesupport").version.to_s, cases: rows) + "\n")
puts JSON.generate(observations: rows.size)
