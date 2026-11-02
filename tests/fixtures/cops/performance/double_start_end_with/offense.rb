text.start_with?('a') || text.start_with?('b')
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/DoubleStartEndWith: Use `text.start_with?('a', 'b')` instead of `text.start_with?('a') || text.start_with?('b')`.
