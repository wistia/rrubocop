factory :user do
  name { "Ada" }
  association :account
  account factory: :account
end
name "Ada"
