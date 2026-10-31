begin; x; rescue A; a; rescue A; b; end
# rrubocop-expect: 1:30 Lint/DuplicateRescueException: Duplicate `rescue` exception detected.
