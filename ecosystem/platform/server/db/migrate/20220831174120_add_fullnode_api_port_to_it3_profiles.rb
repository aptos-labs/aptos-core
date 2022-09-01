# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AddFullnodeApiPortToIt3Profiles < ActiveRecord::Migration[7.0]
  def change
    change_column_null :it3_profiles, :owner_key, true
    change_column_null :it3_profiles, :consensus_key, true
    change_column_null :it3_profiles, :account_key, true
    change_column_null :it3_profiles, :account_address, true
    change_column_null :it3_profiles, :network_key, true

    change_column_null :it3_profiles, :validator_address, true
    change_column_null :it3_profiles, :validator_port, true
    change_column_null :it3_profiles, :validator_metrics_port, true
    change_column_null :it3_profiles, :validator_api_port, true

    add_column :it3_profiles, :fullnode_metrics_port, :integer
    add_column :it3_profiles, :fullnode_api_port, :integer
  end
end
