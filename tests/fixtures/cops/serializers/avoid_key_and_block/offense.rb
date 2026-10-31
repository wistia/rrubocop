attribute :name, key: :display_name do
 object.name
end
# rrubocop-expect: 1:0 Serializers/AvoidKeyAndBlock: Combining key and block could indicate a code error. The attribute field is ignored in this case.
