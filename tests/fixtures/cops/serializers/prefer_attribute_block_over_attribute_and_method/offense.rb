class MaSerializer < ActiveModelSerializers
  attribute :one
  attribute :two
  ^ Serializers/PreferAttributeBlockOverAttributeAndMethod: Prefer block attribute over method call.
  attribute :three

  def two
    "two"
  end
end
