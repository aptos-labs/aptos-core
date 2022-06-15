# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AddExplorerUrlToNft < ActiveRecord::Migration[7.0]
  def change
    add_column :nfts, :explorer_url, :string
  end
end
