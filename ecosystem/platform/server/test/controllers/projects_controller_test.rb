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

  test 'view all projects' do
    3.times do
      FactoryBot.create(:project, user: FactoryBot.create(:user))
    end
    sign_out @user
    get projects_path
    assert_response :success
  end

  test 'view project' do
    sign_out @user
    project = FactoryBot.create(:project, user: @user)
    get project_path(project)
    assert_response :success
  end

  test 'view private project fails if current user is not the creator' do
    project = FactoryBot.create(:project, user: FactoryBot.create(:user), public: false)
    get project_path(project)
    assert_response :forbidden
  end

  test 'new project page' do
    get new_project_path
    assert_response :success
  end

  test 'edit project page' do
    project = FactoryBot.create(:project, user: @user)
    get edit_project_path(project)
    assert_response :success
  end

  test 'edit project page fails if current user is not the creator' do
    project = FactoryBot.create(:project, user: FactoryBot.create(:user))
    get edit_project_path(project)
    assert_response :forbidden
  end

  test 'create new project' do
    category = FactoryBot.create(:category)

    assert_difference('Project.count') do
      post projects_path, params: { project: {
        title: Faker::Company.name,
        short_description: Faker::Company.catch_phrase,
        full_description: Faker::Lorem.paragraphs(number: 3).map { |p| "<p>#{p}</p>" }.join,
        website_url: Faker::Internet.url,
        github_url: Faker::Internet.url(host: 'github.com'),
        discord_url: Faker::Internet.url(host: 'discord.com'),
        twitter_url: Faker::Internet.url(host: 'twitter.com'),
        telegram_url: Faker::Internet.url(host: 't.me'),
        linkedin_url: Faker::Internet.url(host: 'linkedin.com'),
        youtube_url: Faker::Internet.url(host: 'www.youtube.com'),
        thumbnail: Rack::Test::UploadedFile.new('public/favicon.png', 'image/png'),
        category_ids: [category.id],
        project_members_attributes: [
          { user_id: @user.id, role: 'admin', public: true }
        ],
        screenshots: [
          Rack::Test::UploadedFile.new('public/favicon.png', 'image/png')
        ],
        public: true
      } }
    end

    project = Project.last
    assert_redirected_to project_path(project)
    assert_equal @user, project.user
  end

  test 'update existing project' do
    project = FactoryBot.create(:project, user: @user)
    assert_equal true, project.public

    patch project_path(project), params: { project: {
      public: false
    } }
    assert_redirected_to project_path(project)

    project = Project.find(project.id)
    assert_equal false, project.public
  end

  test 'update existing project fails if current user is not the creator' do
    project = FactoryBot.create(:project, user: FactoryBot.create(:user))
    patch project_path(project), params: { project: {
      public: true
    } }
    assert_response :forbidden
  end

  test 'delete existing project' do
    project = FactoryBot.create(:project, user: @user)

    assert_difference('Project.count', -1) do
      delete project_path(project)
    end

    assert_redirected_to projects_path
  end

  test 'delete existing project fails if current user is not the creator' do
    project = FactoryBot.create(:project, user: FactoryBot.create(:user))

    assert_no_difference('Project.count') do
      delete project_path(project)
    end

    assert_response :forbidden
  end
end
