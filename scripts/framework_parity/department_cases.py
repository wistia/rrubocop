"""Original comment scanner, qualification, whitespace and ASCII token semantics."""
def extend(add):
    import json
    from pathlib import Path
    cop='Migration/DepartmentName'
    for row in json.loads((Path(__file__).parents[2]/'tests/fixtures/framework-department-original-spec.json').read_text())['cases']:
        add(cop,row['source'])
    for prefix in ('# rubocop:disable ', '# rubocop:enable ', '# rubocop:todo ', '# rubocop : todo ', '#  rubocop: disable ', '#rubocop:disable '):
        for body in ('Alias','LineLength','Alias, LineLength','Style','all','Style/Alias','SingleSpaceBeforeFirstArg','Alias # reason','Alias -- because','Alias,  LineLength','  Alias','Alias,LineLength','é','é, Alias','Alias, é','Alias, Alias','RSpec','Performance','FactoryBot','Rails','Cops'):
            add(cop,prefix+body)
    for source in ('# rubocop:disable Alias\n# rubocop:enable Alias','# RUBOCOP:disable Alias','# rubocop: disable\tAlias','# rubocop:disable Alias\t,LineLength','# rubocop:disable Alias,\tLineLength','# rubocop:disable Alias, LineLength # comment','name = "# rubocop:disable Alias"'):
        add(cop,source)
    add(cop,'# rubocop:disable Alias',AutoCorrect='disabled')

    for prefix in ('# rubocop:disable ', '# rubocop:enable ', '# rubocop:todo '):
        for body in ('\tAlias','Alias,é,LineLength','Alias,\u00a0LineLength','Alias\u2028LineLength','Alias\u2029LineLength','Alias,\u0085LineLength','\vAlias','Alias   '):
            add(cop,prefix+body)
