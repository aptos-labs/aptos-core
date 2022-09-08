# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AddInitialCategories < ActiveRecord::Migration[7.0]
  def up
    Category.create(title: 'NFTs')
    Category.create(title: 'DeFi')
    Category.create(title: 'Gaming')
    Category.create(title: 'Tooling')
    Category.create(title: 'Wallets')
    Category.create(title: 'Data')
    Category.create(title: 'Lending')
    Category.create(title: 'Other')
  end

  def down
    Category.delete_all
  end
end
