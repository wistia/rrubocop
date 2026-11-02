class User < ApplicationRecord
  def save
  ^ Rails/ActiveRecordOverride: Use `before_save`, `around_save`, or `after_save` callbacks instead of overriding the Active Record method `save`.
    super
  end
end
