# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class CreateIt2Surveys < ActiveRecord::Migration[7.0]
  def change
    create_table :it2_surveys do |t|
      t.references :user, null: false, foreign_key: true
      t.string :persona, null: false
      t.string :participate_reason, null: false
      t.string :qualified_reason, null: false
      t.string :website
      t.string :interest_reason, null: false

      t.timestamps
    end
  end
end
