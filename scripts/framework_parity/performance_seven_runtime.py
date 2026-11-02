"""Trusted Performance programs authored for this audit; no imported specs execute."""
PROGRAMS = [
 ('Performance/RedundantBlockCall', 'def probe(&block)\n if block\n  block.call(1, 2)\n else\n  :none\n end\nend\np [probe { |a,b| a+b }, probe]\n'),
 ('Performance/RedundantBlockCall', 'def probe(&block)\n block.call\nend\nbegin\n probe\nrescue => error\n p error.class.name\nend\n'),
 ('Performance/RedundantBlockCall', 'def probe(&block)\n marker = :outside\n [-> { :inner }].map { |block| block.call }\nend\np probe { :outer }\n'),
 ('Performance/RedundantBlockCall', 'def probe(&block)\n if block\n  block.call\n end\nend\ncallback = proc { :body }\ndef callback.call\n :override\nend\np probe(&callback)\n'),
 ('Performance/RedundantBlockCall', 'def probe(&block)\n if block\n  block.call(\n   __LINE__\n  )\n end\nend\nprobe { |line| p line }\n'),
]

def extend(rows):
    for cop, source, *config in PROGRAMS:
        rows.append(dict(id=f'{cop}:trusted-performance-seven:{len(rows)}', cop=cop,
                         path='app/models/example.rb', source=source, expected=[],
                         corrected=source, config=config[0] if config else {}, trusted_runtime=True))
