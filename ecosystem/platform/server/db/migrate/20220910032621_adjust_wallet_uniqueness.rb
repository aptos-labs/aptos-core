# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AdjustWalletUniqueness < ActiveRecord::Migration[7.0]
  def change
    add_index :wallets, %i[public_key network wallet_name], unique: true
    remove_index :wallets, name: 'index_wallets_on_public_key_and_network'
  end
end
