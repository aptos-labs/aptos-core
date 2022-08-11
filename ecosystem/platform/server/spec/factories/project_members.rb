FactoryBot.define do
  factory :project_member do
    project { nil }
    user { nil }
    role { "MyString" }
  end
end
