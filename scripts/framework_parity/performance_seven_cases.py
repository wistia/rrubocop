"""Original-backed specimens for the seven transferred Performance rules."""
import json
from pathlib import Path

COPS = ('Performance/CompareWithBlock', 'Performance/Detect', 'Performance/RedundantBlockCall',
        'Performance/RedundantMatch', 'Performance/RedundantMerge', 'Performance/RegexpMatch',
        'Performance/StringReplacement')

def extend(record, rows):
    counts = {}
    def add(cop, source, **config):
        record(cop, source, **config)
        counts[cop] = counts.get(cop, 0) + 1
        rows[-1]["id"] = f"{cop}:performance-seven:{counts[cop]}"
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
    for expression in ('!value', '(not value)', '-1.0', '- 1.0', '__ENCODING__', '%W[a#{value}]', '%I[a#{value}]', '%W[a#{build}]', '{missing:}', '{value:}'):
        add('Performance/DoubleStartEndWith', f"text='beta'; value='x'; text.start_with?('a') || text.start_with?({expression})")

    for source in ("h = {}; h.merge!(a:1)", "{}.merge!(a:1)", "{}.merge!({:a => 1})", "h={}; h.merge!(a:1,b:2)", "h={}; h.merge!(a:1,b:h[:a])", "h={}; x=h.merge!(a:1)", "h={}; [h.merge!(a:1)]", "h={}; h.merge!(**other)", "h={}; h.merge!({})", "h={}; h&.merge!(a:1)", "items.each_with_object({}) { |item,h| h.merge!(a:1) }", "items.each_with_object({}) do |item,h|\n marker\n h.merge!(a:1)\nend", "h={}; h.merge!(a:1,b:2) if ready", "h={}; h.merge!(a:1,b:2) while ready", 'h={}; h.merge!("a":1)', 'h={}; h.merge!("":1)', "h={}; a=1; h.merge!(a:)"):
        add('Performance/RedundantMerge', source)
    for limit in (-1, 0, 1, 2, 3, None, False, "1", 1.9):
        add('Performance/RedundantMerge', 'h={}; h.merge!(a:1,b:2)', MaxKeyValuePairs=limit)
    add('Performance/RedundantMerge', '{}.merge!(a:1)', AutoCorrect='disabled')

    for expression in ('a ? b : c', 'a && b', 'a || b', 'a..b', 'method a', 'super a', 'a == b', 'if a then b else c end', 'method(a)', 'method', 'a + b', 'a & b'):
        add('Performance/RedundantMatch', f'something if /regex/.match({expression})')
    for source in ('x.match(y)', 'x.match(/a/,1)', 'x&.match(/a/)', 'p x.match(/a/)', 'case x.match(/a/); when 0; :zero; end', 'x.match(/a/) { |m| work }', '"a".match("a")', '/a/.match("a")', '"a".match(/a/)', 'x.match(/a/) # keep', 'x.match( # keep\n/a/)', 'x.match(/a/) && work', 'def example; x.match(/a/); end', 'def example; x.match(/a/); nil; end'):
        add('Performance/RedundantMatch', source)
    add('Performance/RedundantMatch', '/a/.match("a")', AutoCorrect='disabled')

    for source in ('/a/.match(&block)', 'x.match(/a/, &block)', '/a/.match("alpha") do |m| work end', '/a/.match(<<~TEXT)\nalpha\nTEXT', 'x.match(/a/) if ready', 'while /a/.match("alpha"); break; end', 'begin; work; end while /a/.match("alpha")'):
        add('Performance/RedundantMatch', source)
    add('Performance/RedundantMatch', 'puts :yes if /a/.match("alpha")', TargetRubyVersion=2.7)

    for method, replacement in (('sort','sort_by'), ('sort!','sort_by!'), ('max','max_by'), ('min','min_by'), ('minmax','minmax_by')):
        for left,right in (('a.foo','b.foo'),('a[:foo]','b[:foo]'),("a['foo']","b['foo']"),('a[1]','b[1]'),('b','a')):
            add('Performance/CompareWithBlock', f'array.{method} {{ |a,b| {left} <=> {right} }}')
        add('Performance/CompareWithBlock', f'array.{replacement} {{ |a| a.baz }}')
    for source in ('array.sort { |a,b| a.foo <=> b.bar }', 'array.sort { |a,b| a.foo(1) <=> b.foo(1) }', 'array.sort { |a,b| a[:x] <=> b[:y] }', 'array.sort { |a,b| a[1,2] <=> b[1,2] }', 'array.sort { |a,b| a[value] <=> b[value] }', 'array.sort { |a,b| a&.foo <=> b&.foo }', 'array&.sort { |a,b| a.foo <=> b.foo }', 'array.sort(1) { |a,b| a.foo <=> b.foo }', 'array.sort { |a,b,c| a.foo <=> b.foo }', 'array.sort { |a,b| marker; a.foo <=> b.foo }', 'array.sort do |left,right|\n left.foo <=> right.foo\nend', 'array.sort { |a,b| a.[](0x1) <=> b[1] }', '[[2],[1]].sort { |a,b| a[0] <=> b[0] }', 'array.sort { |a,b| a[1i] <=> b[1i] }'):
        add('Performance/CompareWithBlock', source)
    add('Performance/CompareWithBlock', '[[2],[1]].sort { |a,b| a[0] <=> b[0] }', AutoCorrect='disabled')

    for argument in ('yield a', 'yield'):
        add('Performance/RedundantMatch', f'def probe; something if /regex/.match({argument}); end', TargetRubyVersion=3.2)
    add('Performance/CompareWithBlock', 'array.sort { |a,b| (a.foo {}) <=> (b.foo {}) }')

    for selector in ('select','find_all','filter'):
        for tail in ('.first','.last','[0]','[-1]','.first()','.first(2)','[2]'):
            for block in ('{ true }','{ }','(&:even?)'):
                add('Performance/Detect', f'[1,2,3].{selector}{block}{tail}')
    for source in ('items.select { _1.even? }.first','items.select { |item| item.even? }.first','items.select.first','items.lazy.select { true }.first','lazy.select { true }.first','items&.select { true }.first','items&.select { true }[0]', 'items.select { true }&.first', 'items.select(1) { true }.first'):
        add('Performance/Detect', source)
    add('Performance/Detect', '[1,2].select { true }.first', AutoCorrect='disabled')
    add('Performance/Detect', '[1,2].select { true }.first', GlobalConfig={'Style/CollectionMethods':{'PreferredMethods':{'detect':'find'}}})

    for source in ("'abc'.gsub('a','b')", "'abc'.gsub(/a/,'b'); p $~", "'abc'.gsub(/a/i,'b')", "'abc'.gsub(Regexp.new('a'),'b')", "'abc'.gsub(::Regexp.new('a'),'b')", "'abc'.gsub(Regexp.new('a',0),'b')", "'abc'.gsub(Regexp.compile(/a/),'')", "'abc'.gsub('a','b') { work }", "'abc'.gsub( # keep\n 'a','b')", "gsub('a','b')", "value.gsub('a','b')", "'abc'.gsub(\"a\\n\",'b')", "'abc'.gsub('a',\"\\x01\")", "'abc'.gsub(Regexp.new('\\\\cA'),'b')"):
        add('Performance/StringReplacement', source)
    add('Performance/StringReplacement', "'abc'.gsub('a','b')", AutoCorrect='disabled')
