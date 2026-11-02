1.times { create :user }
3.times { |n| create :user, age: n }
3.times { create :user, age: rand }
3.times { other.create :user }
