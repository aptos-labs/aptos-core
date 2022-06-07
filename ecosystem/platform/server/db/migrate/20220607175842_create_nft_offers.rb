# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class CreateNftOffers < ActiveRecord::Migration[7.0]
  def change
    create_table :nft_offers do |t|
      t.string :name, null: false
      t.datetime :valid_from
      t.datetime :valid_until

      t.timestamps
    end
    add_index :nft_offers, :name, unique: true
  end
end
