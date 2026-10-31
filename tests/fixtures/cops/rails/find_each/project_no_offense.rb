class Example < ApplicationRecord
  def call(includes)
    includes.each { |item| puts item }
  end
end
