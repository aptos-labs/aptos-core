# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class CreateNftImages < ActiveRecord::Migration[7.0]
  def change
    create_table :nft_images do |t|
      t.string :slug, null: false
      t.integer :image_number, null: true

      t.timestamps
    end

    add_index :nft_images, %i[slug image_number], unique: true
  end
end
