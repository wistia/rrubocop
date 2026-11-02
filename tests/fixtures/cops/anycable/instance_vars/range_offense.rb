class Channel; def f; @é = value; @é ||= value; @é; end; end
# rrubocop-expect: 1:22 AnyCable/InstanceVars: Channel instance variables are not supported in AnyCable. Use `state_attr_accessor` instead
# rrubocop-expect: 1:34 AnyCable/InstanceVars: Channel instance variables are not supported in AnyCable. Use `state_attr_accessor` instead
# rrubocop-expect: 1:48 AnyCable/InstanceVars: Channel instance variables are not supported in AnyCable. Use `state_attr_accessor` instead
