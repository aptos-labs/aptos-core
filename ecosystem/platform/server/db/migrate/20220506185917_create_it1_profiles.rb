# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
# frozen_string_literal: true

class CreateIt1Profiles < ActiveRecord::Migration[7.0]
  def change
    create_table :it1_profiles do |t|
      t.references :user, null: false, foreign_key: true, unique: true
      t.string :consensus_key, unique: true
      t.string :account_key, unique: true
      t.string :network_key, unique: true

      t.string :validator_ip
      t.string :validator_address
      t.integer :validator_port
      t.integer :validator_metrics_port
      t.integer :validator_api_port

      t.boolean :validator_verified, default: false

      t.string :fullnode_address
      t.integer :fullnode_port

      t.timestamps
    end
  end
end
