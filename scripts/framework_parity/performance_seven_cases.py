"""Original-backed specimens for the seven transferred Performance rules."""
import json
from pathlib import Path

COPS = ('Performance/CompareWithBlock', 'Performance/Detect', 'Performance/RedundantBlockCall',
        'Performance/RedundantMatch', 'Performance/RedundantMerge', 'Performance/RegexpMatch',
        'Performance/StringReplacement')

def extend(add):
    source = Path(__file__).resolve().parents[2] / 'tests/fixtures/performance-seven-upstream/cases.json'
    if source.exists():
        for row in json.loads(source.read_text()):
            add(row['cop'], row['source'].rstrip('\n'), **row.get('config', {}))
    for call in ('block.call', 'block.call()', 'block.call(1, 2)', 'block.call 1,2', 'block.(1)', 'block::call(1)', 'block&.call(1)', 'block[1]', 'other.call', 'block.call(a:1,b:2)', 'block.call({a:1,b:2})', 'block.call(1, a: 2, b: 3)', 'block.call(&other)', 'block.call { work }'):
        for guard in (False, True):
            body = f'if block\n {call}\nend' if guard else call
            add('Performance/RedundantBlockCall', f'def example(&block)\n{body}\nend')
    for source in ('def example(&block); if block_given?; block.call(1); end; end',
                   'def example(&block); block && block.call(1); end',
                   'def example(&block); !block || block.call(1); end',
                   'def example(&block); block.call(1) if block; end',
                   'def example(&block); unless !block; block.call(1); end; end',
                   'def example(&block); block.call unless block; end',
                   'def example(&block); block = other; block.call; end',
                   'def example(&block); block ||= other; block.call; end',
                   'def example(&block); block, other = values; block.call; end',
                   'def example(&block); for block in items; block.call; end; end',
                   'def example(&block); items.each { |block| block.call }; end',
                   'def example(&block); marker; items.each { |block| block.call }; end',
                   'def example(&block); if block; items.each { |value| block.call(value) }; end; end',
                   'def example(&block); if block; block.call( # keep\n 1\n ); end; end',
                   'def example(&block); block.call(1); block.call(2, &other); end',
                   'def example(&block); block.call(1); block.call(2) { work }; end',
                   'def example(&block); def nested(block); block.call; end; end',
                   'def example(&block); def nested(&block); if block; block.call; end; end; end',
                   'def example(&block); if block; value => block; block.call; end; end',
                   'def example(&block); block.call = value; end',
                   'def example(&block); if block; block.call(<<~TEXT); end\nhello\nTEXT\nend'):
        add('Performance/RedundantBlockCall', source)
    add('Performance/RedundantBlockCall', 'def example(&block); if block; block.call; end; end', AutoCorrect='disabled')
    add('Performance/RedundantBlockCall', 'def example(&block); if block; block.call; block.call = value; end; end')

    add('Performance/RedundantBlockCall', 'def example(&block)\n if block\n  block.call(\n   __LINE__\n  )\n end\nend')
    add('Performance/RedundantBlockCall', 'def example(&block)\n if block\n  block.call(\n   <<~TEXT\n    payload\n   TEXT\n  )\n end\nend')
