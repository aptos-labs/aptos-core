# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AddPersonasToUser < ActiveRecord::Migration[6.1]
  def change
    change_table :users do |t|
      t.boolean :is_developer, null: false, default: false
      t.boolean :is_node_operator, null: false, default: false

      t.string :mainnet_address, null: true
      t.string :kyc_status, null: false, default: :not_started
    end
  end
end
