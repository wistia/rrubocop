key = :display_name
attribute(:name, key:) { 'value' }
# rrubocop-expect: 2:0 Serializers/AvoidKeyAndBlock: Combining key and block could indicate a code error. The attribute field is ignored in this case.
