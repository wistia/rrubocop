"""Production and excluded-path CLI controls for transferred Performance rules."""
def rows(model):
    return [
        ('Performance/RegexpMatch', 'puts :yes if /a/.match("alpha")', model, 'vendor/performance_regexp_scope.rb'),
        ('Performance/StringReplacement', '"abc".gsub("a","b")', model, 'vendor/performance_string_scope.rb'),
        ('Performance/Detect', '[1,2].select { true }.first', model, 'vendor/performance_detect_scope.rb'),
        ('Performance/CompareWithBlock', '[[2],[1]].sort { |a,b| a[0] <=> b[0] }', model, 'vendor/performance_compare_scope.rb'),
        ('Performance/RedundantMatch', 'puts :yes if /a/.match("alpha")', model, 'vendor/performance_match_scope.rb'),
        ('Performance/RedundantMerge', 'hash = {}; hash.merge!(a: 1)', model, 'vendor/performance_merge_scope.rb'),
        ('Performance/RedundantBlockCall', 'def example(&block); if block; block.call; end; end',
         model, 'vendor/performance_seven_scope.rb'),
    ]
