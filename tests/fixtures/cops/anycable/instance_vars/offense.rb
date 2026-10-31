class Channel; def subscribed; @post = find; @post; end; end
# rrubocop-expect: 1:31 AnyCable/InstanceVars: Channel instance variables are not supported in AnyCable. Use `state_attr_accessor` instead
# rrubocop-expect: 1:45 AnyCable/InstanceVars: Channel instance variables are not supported in AnyCable. Use `state_attr_accessor` instead
