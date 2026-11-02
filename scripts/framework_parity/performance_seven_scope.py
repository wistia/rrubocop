"""Production and excluded-path CLI controls for transferred Performance rules."""
def rows(model):
    return [
        ('Performance/CompareWithBlock', '[[2],[1]].sort { |a,b| a[0] <=> b[0] }', model, 'vendor/performance_compare_scope.rb'),
        ('Performance/RedundantMatch', 'puts :yes if /a/.match("alpha")', model, 'vendor/performance_match_scope.rb'),
        ('Performance/RedundantMerge', 'hash = {}; hash.merge!(a: 1)', model, 'vendor/performance_merge_scope.rb'),
        ('Performance/RedundantBlockCall', 'def example(&block); if block; block.call; end; end',
         model, 'vendor/performance_seven_scope.rb'),
    ]
