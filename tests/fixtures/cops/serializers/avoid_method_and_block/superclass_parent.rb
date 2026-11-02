class S < Class.new { def title; 'x'; end }
 attribute(:title) { 'value' }
end
# rrubocop-expect: 2:1 Serializers/AvoidMethodAndBlock: Combining method and block could indicate a code error. The method is ignored in this case.
