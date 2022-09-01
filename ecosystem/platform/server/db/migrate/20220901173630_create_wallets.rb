# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class CreateWallets < ActiveRecord::Migration[7.0]
  def change
    create_table :wallets do |t|
      t.references :user, null: false, foreign_key: true
      t.string :network, null: false, comment: 'The network that the account exists on (e.g. "ait3").'
      t.string :public_key, null: false, comment: 'The public key of the account.'
      t.boolean :verified, null: false, default: false,
                           comment: 'Whether the user has been verified as the owner of this wallet.'

      t.timestamps

      t.index %i[public_key network], unique: true
      t.check_constraint "public_key ~ '^0x[0-9a-f]{64}$'"
    end
  end
end
