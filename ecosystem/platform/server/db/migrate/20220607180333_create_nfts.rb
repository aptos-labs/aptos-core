# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class CreateNfts < ActiveRecord::Migration[7.0]
  def change
    create_table :nfts do |t|
      t.references :user, null: false, foreign_key: true
      t.references :nft_offer, null: false, foreign_key: true

      t.timestamps
    end
  end
end
