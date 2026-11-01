0.times { work }; keep_me
^^^^^^^^^^^^^^^^ Lint/UselessTimes: Useless call to `0.times` detected.
1.times { |i| def nested; i; end }
^ Lint/UselessTimes: Useless call to `1.times` detected.
1.times { |i| case [42]; in [i]; puts i; end }
^ Lint/UselessTimes: Useless call to `1.times` detected.

def returns_count
  1.times { :value }
  ^^^^^^^^^^^^^^^^^^ Lint/UselessTimes: Useless call to `1.times` detected.
end
