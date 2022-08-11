require 'rails_helper'

RSpec.describe "projects/edit", type: :view do
  before(:each) do
    @project = assign(:project, Project.create!(
      title: "MyString",
      short_description: "MyString",
      full_description: "MyText",
      website_url: "MyString",
      github_url: "MyString",
      discord_url: "MyString",
      twitter_url: "MyString",
      telegram_url: "MyString",
      linkedin_url: "MyString",
      thumbnail_url: "MyString",
      youtube_url: "MyString",
      forum_url: "MyString",
      public: false
    ))
  end

  it "renders the edit project form" do
    render

    assert_select "form[action=?][method=?]", project_path(@project), "post" do

      assert_select "input[name=?]", "project[title]"

      assert_select "input[name=?]", "project[short_description]"

      assert_select "textarea[name=?]", "project[full_description]"

      assert_select "input[name=?]", "project[website_url]"

      assert_select "input[name=?]", "project[github_url]"

      assert_select "input[name=?]", "project[discord_url]"

      assert_select "input[name=?]", "project[twitter_url]"

      assert_select "input[name=?]", "project[telegram_url]"

      assert_select "input[name=?]", "project[linkedin_url]"

      assert_select "input[name=?]", "project[thumbnail_url]"

      assert_select "input[name=?]", "project[youtube_url]"

      assert_select "input[name=?]", "project[forum_url]"

      assert_select "input[name=?]", "project[public]"
    end
  end
end
