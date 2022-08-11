require 'rails_helper'

RSpec.describe "projects/index", type: :view do
  before(:each) do
    assign(:projects, [
      Project.create!(
        title: "Title",
        short_description: "Short Description",
        full_description: "MyText",
        website_url: "Website Url",
        github_url: "Github Url",
        discord_url: "Discord Url",
        twitter_url: "Twitter Url",
        telegram_url: "Telegram Url",
        linkedin_url: "Linkedin Url",
        thumbnail_url: "Thumbnail Url",
        youtube_url: "Youtube Url",
        forum_url: "Forum Url",
        public: false
      ),
      Project.create!(
        title: "Title",
        short_description: "Short Description",
        full_description: "MyText",
        website_url: "Website Url",
        github_url: "Github Url",
        discord_url: "Discord Url",
        twitter_url: "Twitter Url",
        telegram_url: "Telegram Url",
        linkedin_url: "Linkedin Url",
        thumbnail_url: "Thumbnail Url",
        youtube_url: "Youtube Url",
        forum_url: "Forum Url",
        public: false
      )
    ])
  end

  it "renders a list of projects" do
    render
    assert_select "tr>td", text: "Title".to_s, count: 2
    assert_select "tr>td", text: "Short Description".to_s, count: 2
    assert_select "tr>td", text: "MyText".to_s, count: 2
    assert_select "tr>td", text: "Website Url".to_s, count: 2
    assert_select "tr>td", text: "Github Url".to_s, count: 2
    assert_select "tr>td", text: "Discord Url".to_s, count: 2
    assert_select "tr>td", text: "Twitter Url".to_s, count: 2
    assert_select "tr>td", text: "Telegram Url".to_s, count: 2
    assert_select "tr>td", text: "Linkedin Url".to_s, count: 2
    assert_select "tr>td", text: "Thumbnail Url".to_s, count: 2
    assert_select "tr>td", text: "Youtube Url".to_s, count: 2
    assert_select "tr>td", text: "Forum Url".to_s, count: 2
    assert_select "tr>td", text: false.to_s, count: 2
  end
end
