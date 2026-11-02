class User < ApplicationRecord
  def save
    super()
  end
end
class Other < Base
  def save
    super
  end
end
