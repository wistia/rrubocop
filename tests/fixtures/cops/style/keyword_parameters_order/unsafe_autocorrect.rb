def b; :method; end
def m(a: b, b:)
      ^ Style/KeywordParametersOrder: Place optional keyword parameters at the end of the parameters list.
 a
end
p m(b: :arg)
