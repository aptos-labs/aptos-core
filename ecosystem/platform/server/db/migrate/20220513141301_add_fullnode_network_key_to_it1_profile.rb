# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
class AddFullnodeNetworkKeyToIt1Profile < ActiveRecord::Migration[7.0]
  def change
    add_column :it1_profiles, :fullnode_network_key, :string, unique: true
  end
end
