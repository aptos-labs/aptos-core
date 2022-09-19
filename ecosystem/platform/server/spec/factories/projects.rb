# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

FactoryBot.define do
  factory :project do
    user
    title { Faker::Company.name }
    short_description { Faker::Company.catch_phrase }
    full_description { Faker::Lorem.paragraphs(number: 3).join("\n\n") }
    website_url { Faker::Internet.url }
    github_url { Faker::Internet.url(host: 'github.com') }
    discord_url { Faker::Internet.url(host: 'discord.com') }
    twitter_url { Faker::Internet.url(host: 'twitter.com') }
    telegram_url { Faker::Internet.url(host: 't.me') }
    linkedin_url { Faker::Internet.url(host: 'linkedin.com') }
    youtube_url { Faker::Internet.url(host: 'www.youtube.com') }
    thumbnail { Rack::Test::UploadedFile.new('app/assets/images/favicon.png', 'image/png') }
    public { true }
    verified { true }
    project_categories { build_list(:project_category, 3, project: instance).uniq(&:category_id) }
    project_members { build_list(:project_member, 3, project: instance) }
    screenshots { 4.times.map { |_| Rack::Test::UploadedFile.new('app/assets/images/favicon.png', 'image/png') } }
  end
end
