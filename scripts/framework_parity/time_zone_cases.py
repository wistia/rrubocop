"""Pinned TimeZone const callbacks, literal values, hash offsets and chain traversal."""
def extend(add):
    import json
    from pathlib import Path
    cop='Rails/TimeZone'
    for row in json.loads((Path(__file__).parents[2]/'tests/fixtures/framework-time-zone-original-spec.json').read_text())['cases']:
        add(cop,row['source'],EnforcedStyle=row['style'])
    for style in ('strict','flexible'):
        for receiver in ('Time','::Time',':: Time','Other::Time','(Time)'):
            for method in ('now','new','local','at','parse','current','zone','class'):
                for suffix in ('','.utc','.in_time_zone','.localtime',".localtime('+03:00')",'.now','.parse'):
                    add(cop,f'{receiver}.{method}{suffix}',EnforcedStyle=style)
        for method in ('now','new','local','at','parse'):
            for arg in ('in: nil','in: false','in: "+03:00"','"in" => nil','"in": nil','{:in=>false}','1,2,3,4,5,6,7','1,2,3,4,5,6,7,8',':UTC','"March"','"Z\\n"','1e999','1i','__FILE__','__LINE__'):
                add(cop,f'Time.{method}({arg})',EnforcedStyle=style)
        for receiver in ('"2020-01-02"','"2020-01-02Z"','"2020\\n01"','"a" "b"','?Z','__FILE__',':time'):
            for op in ('.','&.'):
                add(cop,receiver+op+'to_time',EnforcedStyle=style)
        for source in ('Time&.now','Time.now&.utc','Time.now { work }.utc','wrap(Time.now).utc','Time.now(Time.at(0))','Time.now(Time.new)','Time[0].now','Time.now(Time + 1)','Time.now = value','Time.now # rubocop:disable Rails/TimeZone'):
            add(cop,source,EnforcedStyle=style)
    add(cop,'Time.now',AutoCorrect='disabled')
