# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class Project < ApplicationRecord
  URL_FORMAT = URI::DEFAULT_PARSER.make_regexp(%w[http https])
  VALID_HOSTS = {
    github_url: 'github.com',
    discord_url: 'discord.com',
    twitter_url: 'twitter.com',
    telegram_url: 't.me',
    linkedin_url: 'linkedin.com',
    youtube_url: 'www.youtube.com',
    forum_url: 'forum.aptoslabs.com'
  }.freeze

  has_many :categories, through: :project_categories, dependent: :destroy
  has_many :members, through: :project_members, dependent: :destroy
  has_many :milestones, class_name: 'ProjectMilestone', dependent: :destroy
  has_many :screenshots, class_name: 'ProjectScreenshot', dependent: :destroy

  validates :title, presence: true, length: { maximum: 140 }
  validates :short_description, presence: true, length: { maximum: 140 }
  validates :full_description, presence: true, length: { minimum: 140 }
  validates :website_url, presence: true, format: URL_FORMAT
  validates :thumbnail_url, presence: true, format: URL_FORMAT
  validates :github_url, format: URL_FORMAT, allow_nil: true, allow_blank: true
  validates :discord_url, format: URL_FORMAT, allow_nil: true, allow_blank: true
  validates :twitter_url, format: URL_FORMAT, allow_nil: true, allow_blank: true
  validates :telegram_url, format: URL_FORMAT, allow_nil: true, allow_blank: true
  validates :linkedin_url, format: URL_FORMAT, allow_nil: true, allow_blank: true
  validates :youtube_url, format: URL_FORMAT, allow_nil: true, allow_blank: true
  validates :forum_url, format: URL_FORMAT, allow_nil: true, allow_blank: true
  validates_each VALID_HOSTS.keys do |record, attr, value|
    host = VALID_HOSTS[attr]
    record.errors.add(attr, "must point to #{host}") unless URI.parse(value).host == host
  end
  validates :categories, length: { minimum: 1, maximum: 4 }
  validates :project_screenshots, length: { maximum: 5 }
end
