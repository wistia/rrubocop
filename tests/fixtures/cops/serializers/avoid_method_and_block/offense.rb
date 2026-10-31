class Ser
 attribute :name do; object.name; end
 def name; "n"; end
end
# rrubocop-expect: 2:1 Serializers/AvoidMethodAndBlock: Combining method and block could indicate a code error. The method is ignored in this case.
