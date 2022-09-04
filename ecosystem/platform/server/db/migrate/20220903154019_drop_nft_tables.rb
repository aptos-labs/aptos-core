# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class DropNftTables < ActiveRecord::Migration[7.0]
  def up
    drop_table :nfts
    drop_table :nft_offers
  end

  def down
    raise ActiveRecord::IrreversibleMigration
  end
end
