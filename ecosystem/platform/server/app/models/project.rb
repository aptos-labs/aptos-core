# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class Project < ApplicationRecord
  include PgSearch::Model

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

  belongs_to :user
  has_many :project_categories, dependent: :destroy
  has_many :categories, through: :project_categories
  has_many :project_members, dependent: :destroy
  has_many :members, through: :project_members, source: :user do
    def public
      where('project_members.public': true)
    end
  end
  has_one_attached :thumbnail
  has_many_attached :screenshots
  accepts_nested_attributes_for :project_categories, :project_members

  scope :filter_by_category, lambda { |category_id|
    joins(:project_categories).where('project_categories.category_id': category_id)
  }

  validates :title, presence: true, length: { maximum: 140 }
  validates :short_description, presence: true, length: { maximum: 140 }
  validates :full_description, presence: true, length: { minimum: 140 }
  validates :website_url, presence: true, format: URL_FORMAT
  validates :github_url, format: URL_FORMAT, allow_nil: true
  validates :discord_url, format: URL_FORMAT, allow_nil: true
  validates :twitter_url, format: URL_FORMAT, allow_nil: true
  validates :telegram_url, format: URL_FORMAT, allow_nil: true
  validates :linkedin_url, format: URL_FORMAT, allow_nil: true
  validates :youtube_url, format: URL_FORMAT, allow_nil: true
  validates :forum_url, format: URL_FORMAT, allow_nil: true
  validates_each VALID_HOSTS.keys, allow_nil: true do |record, attr, value|
    host = VALID_HOSTS[attr]
    record.errors.add(attr, "must point to #{host}") unless URI.parse(value).host == host
  end
  validates :project_categories, length: { minimum: 1, maximum: 4 }
  validates :screenshots, length: { minimum: 1, maximum: 5 }

  pg_search_scope :search,
                  against: %i[title short_description full_description],
                  using: { tsearch: { dictionary: 'english' } }
end
