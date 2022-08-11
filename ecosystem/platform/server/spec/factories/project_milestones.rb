FactoryBot.define do
  factory :project_milestone do
    project { nil }
    title { "MyString" }
    start_date { "2022-08-11" }
    end_date { "2022-08-11" }
  end
end
