"""Production and excluded-path CLI controls for transferred Performance rules."""
def rows(model):
    return [
        ('Performance/RedundantBlockCall', 'def example(&block); if block; block.call; end; end',
         model, 'vendor/performance_seven_scope.rb'),
    ]
