# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'
require 'mocha/minitest'

class ProjectsControllerTest < ActionDispatch::IntegrationTest
  include Devise::Test::IntegrationHelpers

  setup do
    @user = FactoryBot.create(:user)
    sign_in @user
    Flipper.enable(:projects)
    ProjectsController.any_instance.stubs(:verify_recaptcha).returns(true)
  end

  test 'new project page' do
    get new_project_path
    assert_response :success
  end

  test 'create new project' do
    category = Category.create(title: Faker::Company.buzzword)

    assert_difference('Project.count') do
      post projects_path, params: { project: {
        title: Faker::Company.name,
        short_description: Faker::Company.catch_phrase,
        full_description: Faker::Lorem.paragraphs(number: 3).join("\n\n"),
        website_url: Faker::Internet.url,
        github_url: Faker::Internet.url(host: 'github.com'),
        discord_url: Faker::Internet.url(host: 'discord.com'),
        twitter_url: Faker::Internet.url(host: 'twitter.com'),
        telegram_url: Faker::Internet.url(host: 't.me'),
        linkedin_url: Faker::Internet.url(host: 'linkedin.com'),
        youtube_url: Faker::Internet.url(host: 'www.youtube.com'),
        thumbnail_url: Faker::Company.logo,
        project_categories_attributes: [
          { category_id: category.id }
        ],
        project_members_attributes: [
          { user_id: @user.id, role: 'admin', public: true }
        ],
        project_screenshots_attributes: [
          { url: Faker::LoremPixel.image(size: '1920x1080') }
        ],
        public: true
      } }
    end

    assert_redirected_to project_path(Project.last)
  end
end
