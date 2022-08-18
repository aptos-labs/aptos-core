# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class CreateProjectCategories < ActiveRecord::Migration[7.0]
  def change
    create_table :project_categories do |t|
      t.references :project, null: false, foreign_key: true
      t.references :category, null: false, foreign_key: true

      t.timestamps
    end

    add_index :project_categories, %i[category_id project_id], unique: true
  end
end
