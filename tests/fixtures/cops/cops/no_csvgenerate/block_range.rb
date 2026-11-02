CSV.generate { |csv| csv << row }
# rrubocop-expect: 1:0 Cops/NoCSVGenerate: Avoid using `CSV.generate`. Please use `CSV.safe_generate` instead to avoid excel macro attacks.
