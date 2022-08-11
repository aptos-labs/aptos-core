FactoryBot.define do
  factory :project do
    title { "MyString" }
    short_description { "MyString" }
    full_description { "MyText" }
    website_url { "MyString" }
    github_url { "MyString" }
    discord_url { "MyString" }
    twitter_url { "MyString" }
    telegram_url { "MyString" }
    linkedin_url { "MyString" }
    thumbnail_url { "MyString" }
    youtube_url { "MyString" }
    forum_url { "MyString" }
    public { false }
  end
end
