# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class CreateProjectMilestones < ActiveRecord::Migration[7.0]
  def change
    create_table :project_milestones do |t|
      t.references :project, null: false, foreign_key: true
      t.string :title, null: false
      t.date :start_date
      t.date :end_date

      t.timestamps
    end
  end
end
