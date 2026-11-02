class S
 def title; 'x'; end
 attribute(
 # name
 :title) { 'y' }
end
# rrubocop-expect: 3:1 Serializers/AvoidMethodAndBlock: Combining method and block could indicate a code error. The method is ignored in this case.
