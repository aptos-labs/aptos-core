# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class CreateIt2Profiles < ActiveRecord::Migration[7.0]
  def change
    create_table :it2_profiles do |t|
      t.references :user, null: false, foreign_key: true, index: { unique: true }
      t.string :consensus_key, null: false, index: { unique: true }
      t.string :account_key, null: false, index: { unique: true }
      t.string :network_key, null: false, index: { unique: true }
      t.string :validator_ip
      t.string :validator_address, null: false
      t.integer :validator_port, null: false
      t.integer :validator_metrics_port, null: false
      t.integer :validator_api_port, null: false
      t.boolean :validator_verified, default: false, null: false
      t.string :fullnode_address
      t.integer :fullnode_port
      t.string :fullnode_network_key, index: { unique: true }
      t.boolean :terms_accepted, default: false, null: false
      t.boolean :selected, default: false, null: false,
                           comment: 'Whether this node is selected for participation in IT2.'
      t.boolean :validator_verified_final
      t.jsonb :metrics_data

      t.timestamps
    end
  end
end
