# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class CreateNetworkOperations < ActiveRecord::Migration[7.0]
  def change
    create_table :network_operations do |t|
      t.string :title, null: false
      t.text :content, null: false

      t.timestamps null: true
    end
  end
end
