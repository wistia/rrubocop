"""Trusted custom delegation and actual pinned ActiveSupport option contracts."""
def programs():
    result=[]
    for options in ('allow_blank: true', ':allow_blank => true', '"allow_blank": true', 'allow_blank: false', 'allow_nil: true', 'allow_blank: true, allow_nil: false'):
        result.append(('Rails/DelegateAllowBlank', "def delegate(*args, **options); p [args, options]; end\ndelegate :name, "+options+"\n"))
    for options in ('allow_blank: true', 'allow_blank: false', 'allow_nil: true'):
        result.append(('Rails/DelegateAllowBlank', "gem 'activesupport','8.1.3.1'\nrequire 'active_support/core_ext/module/delegation'\nbegin\nclass Probe\ndelegate :name, to: :account, "+options+"\nend\nputs :defined\nrescue ArgumentError => error\np error.class\nend\n"))
    return result
