# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class CreateNfts < ActiveRecord::Migration[7.0]
  def change
    create_table :nfts do |t|
      t.references :user, null: false, foreign_key: true
      t.references :nft_offer, null: false, foreign_key: true
      t.string :image_url, null: false, comment: 'The image that the NFT points to.'

      t.timestamps
    end
    add_index :nfts, :image_url, unique: true
  end
end
