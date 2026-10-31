class MaSerializer < ActiveModelSerializers
  attributes :one,
    :two,
    ^ Serializers/PreferAttributeBlockOverAttributesAndMethods: Prefer block attribute over method call from attributes.
    :three

  def two
    "two"
  end
end
