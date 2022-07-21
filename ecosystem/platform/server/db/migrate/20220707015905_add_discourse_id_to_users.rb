# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AddDiscourseIdToUsers < ActiveRecord::Migration[7.0]
  def change
    add_column :users, :discourse_id, :integer, null: true, index: { unique: true }
  end
end
