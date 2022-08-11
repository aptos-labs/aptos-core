# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class CreateProjects < ActiveRecord::Migration[7.0]
  def change
    create_table :projects do |t|
      t.string :title, null: false
      t.string :short_description, null: false
      t.string :full_description, null: false
      t.string :website_url, null: false
      t.string :thumbnail_url, null: false
      t.string :github_url
      t.string :discord_url
      t.string :twitter_url
      t.string :telegram_url
      t.string :linkedin_url
      t.string :youtube_url
      t.string :forum_url
      t.boolean :public, null: false

      t.timestamps
    end
  end
end
