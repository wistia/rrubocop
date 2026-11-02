skip_before_action :login, only: :show, if: :trusted?
                                        ^^^^^^^^^^^^^ Rails/IgnoredSkipActionFilterOption: `if` option will be ignored when `only` and `if` are used together.
