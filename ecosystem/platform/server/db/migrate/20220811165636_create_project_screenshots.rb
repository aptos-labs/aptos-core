# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class CreateProjectScreenshots < ActiveRecord::Migration[7.0]
  def change
    create_table :project_screenshots do |t|
      t.references :project, null: false, foreign_key: true
      t.string :url, null: false

      t.timestamps
    end
  end
end
