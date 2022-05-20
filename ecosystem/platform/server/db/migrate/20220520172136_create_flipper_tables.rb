# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
class CreateFlipperTables < ActiveRecord::Migration[7.0]
  def self.up
    create_table :flipper_features do |t|
      t.string :key, null: false
      t.timestamps null: false
    end
    add_index :flipper_features, :key, unique: true

    create_table :flipper_gates do |t|
      t.string :feature_key, null: false
      t.string :key, null: false
      t.string :value
      t.timestamps null: false
    end
    add_index :flipper_gates, %i[feature_key key value], unique: true
  end

  def self.down
    drop_table :flipper_gates
    drop_table :flipper_features
  end
end
