"""Trusted Performance programs authored for this audit; no imported specs execute."""
PROGRAMS = [
 ('Performance/RedundantBlockCall', 'def probe(&block)\n if block\n  block.call(1, 2)\n else\n  :none\n end\nend\np [probe { |a,b| a+b }, probe]\n'),
 ('Performance/RedundantBlockCall', 'def probe(&block)\n block.call\nend\nbegin\n probe\nrescue => error\n p error.class.name\nend\n'),
 ('Performance/RedundantBlockCall', 'def probe(&block)\n marker = :outside\n [-> { :inner }].map { |block| block.call }\nend\np probe { :outer }\n'),
 ('Performance/RedundantBlockCall', 'def probe(&block)\n if block\n  block.call\n end\nend\ncallback = proc { :body }\ndef callback.call\n :override\nend\np probe(&callback)\n'),
 ('Performance/RedundantBlockCall', 'def probe(&block)\n if block\n  block.call(\n   __LINE__\n  )\n end\nend\nprobe { |line| p line }\n'),
 ('Performance/RedundantMerge', 'h={}; h.merge!(a:1,b:h[:a]); p h\n'),
 ('Performance/RedundantMerge', 'class Receiver; def merge!(other); puts :merge; end; def []=(key,value); puts :store; end; end; receiver=Receiver.new; receiver.merge!(a:1)\n'),
 ('Performance/RedundantMerge', 'p({}.merge!(a:1)); {}.merge!(a:1)\n'),
 ('Performance/RedundantMatch', 'case "alpha".match(/a/); when MatchData; puts :match_data; when Integer; puts :integer; end\n'),
 ('Performance/RedundantMatch', 'class Receiver; def match(pattern); puts :match; true; end; def =~(pattern); puts :operator; false; end; end; receiver=Receiver.new; puts :yes if receiver.match(/a/)\n'),
 ('Performance/RedundantMatch', 'puts :yes if /a/.match("alpha"); p [$~[0],$&]\n'),
]

def extend(rows):
    for index, (cop, source, *config) in enumerate(PROGRAMS):
        rows.append(dict(id=f'{cop}:trusted-performance-seven:{index}', cop=cop,
                         path='app/models/example.rb', source=source, expected=[],
                         corrected=source, config=config[0] if config else {}, trusted_runtime=True))
