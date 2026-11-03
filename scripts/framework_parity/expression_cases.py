"""Original legacy-AST grouping cases, including delayed literals and typed numbers."""
def extend(add):
    pairs = [
        ('/a/im', '/a/mi'), ('/a/i', '/a/m'), ('/a/', '%r(a)'),
        (r'/a\/b/', '%r(a/b)'), (r'%r(\()', r'/\(/'),
        (r'%r([\(])', '/[(]/'), (r'%r([\(])', r'/[\(]/'),
        ('/#{pattern}/', '%r(#{pattern})'), ('/#{pattern}/', '/#{other}/'),
        ('1r', '1.0r'), ('0r', '0.0r'), ('- -1r', '1r'),
        ('1i', '1.0i'), ('-1i', '-0x1i'), ('1ri', '1i'),
        ('+1.0', '1.0'), ('-0.0', '0.0'), ('1+2', '1.+(2)'),
        ('!value', 'value.!'), ('not(value)', '!value'),
        ('value && other', 'value and other'),
        ('object.foo = 1', 'object.foo=(1)'),
        ('object[0] = 1', 'object.[]=(0, 1)'), ('object[0]', 'object.[](0)'),
        ('[]', '[ ]'), ('proc {}', 'proc do end'),
        ('proc { work }', 'proc { || work }'),
        ("'a' 'b'", "'ab'"), ("'a' 'b'", '"a" "b"'),
        ('"a\nb"', r'"a\nb"'), ('"a\nb"', '%q(a\nb)'),
        ("'a\nb' 'c'", "'a' 'b\nc'"), ('?a', "'a'"),
    ]
    for left, right in pairs:
        add('Rails/EnumUniqueness', f'enum :status, [({left}), ({right})]')
    for first, second in [('alpha\nbeta\n', 'alpha\nbeta\n'),
                          ('alpha\nbeta\n', 'gamma\ndelta\n'),
                          ('#{value}\nnext\n', '#{value}\nnext\n'),
                          ('#{value}\nnext\n', '#{other}\nnext\n'),
                          ('', ''), ('alpha\\\nbeta\n', 'alphabeta\n')]:
        add('Rails/EnumUniqueness', f'enum :status, [<<~TEXT, <<~TEXT]\n{first}TEXT\n{second}TEXT')
    for body, literal in [('alpha\n', r'"alpha\n"'),
                          ('alpha\nbeta\n', r'"alpha\nbeta\n"'), ('', '""')]:
        add('Rails/EnumUniqueness', f'enum :status, [<<~TEXT, {literal}]\n{body}TEXT')
