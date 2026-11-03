"""Pinned DelegateAllowBlank matcher arity, symbols, literal truth, and dispatch roles."""
def extend(add):
    import json
    from pathlib import Path
    for row in json.loads((Path(__file__).parents[2]/"tests/fixtures/framework-delegate-blank-original-spec.json").read_text())["cases"]:
        add("Rails/DelegateAllowBlank",row["source"])
    for prefix in ('delegate', 'self.delegate', 'other.delegate', 'other&.delegate'):
        for methods in ('', ':name, ', ':name, :email, ', 'names, '):
            for option in ('allow_blank: true', 'allow_blank: false', 'allow_blank: nil', 'allow_blank: value', ':allow_blank => true', '"allow_blank" => true', '"allow_blank": true', 'allow_nil: true', '**options', 'allow_blank: true, to: :account'):
                add('Rails/DelegateAllowBlank',f'{prefix}({methods}{option})')
    for source in ('delegate :name, {allow_blank: true}', 'delegate :name, to: :account, allow_blank: true', 'delegate :name, allow_blank: true, allow_nil: false', 'delegate :name, allow_blank: true, allow_blank: false', 'delegate :name, allow_blank: false, allow_blank: true', 'delegate :name, **options, allow_blank: true', 'delegate :name, allow_blank: true, **options', 'delegate :name, allow_blank: (true)', 'delegate(:name, allow_blank: true) { _1 }', 'delegate(:name, allow_blank: true) { work }', 'delegate = true; delegate', 'def delegate; end', 'delegate :name, allow_blank: true # rubocop:disable Rails/DelegateAllowBlank', 'delegate :name, allow_blank:\n true'):
        add('Rails/DelegateAllowBlank',source)
    add('Rails/DelegateAllowBlank','delegate :name, allow_blank: true',AutoCorrect='disabled')
