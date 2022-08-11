require 'rails_helper'

RSpec.describe "projects/show", type: :view do
  before(:each) do
    @project = assign(:project, Project.create!(
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
    ))
  end

  it "renders attributes in <p>" do
    render
    expect(rendered).to match(/Title/)
    expect(rendered).to match(/Short Description/)
    expect(rendered).to match(/MyText/)
    expect(rendered).to match(/Website Url/)
    expect(rendered).to match(/Github Url/)
    expect(rendered).to match(/Discord Url/)
    expect(rendered).to match(/Twitter Url/)
    expect(rendered).to match(/Telegram Url/)
    expect(rendered).to match(/Linkedin Url/)
    expect(rendered).to match(/Thumbnail Url/)
    expect(rendered).to match(/Youtube Url/)
    expect(rendered).to match(/Forum Url/)
    expect(rendered).to match(/false/)
  end
end
