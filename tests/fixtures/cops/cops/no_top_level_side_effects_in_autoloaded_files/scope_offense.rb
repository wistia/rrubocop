flag ? Foo.run : Bar.run
# rrubocop-expect: 1:0 Cops/NoTopLevelSideEffectsInAutoloadedFiles: Top-level code in an autoloaded file runs at boot when eager loaded. Only define constants here; expose a `self.run` and invoke it via `bin/rails runner 'Klass.run'`.
