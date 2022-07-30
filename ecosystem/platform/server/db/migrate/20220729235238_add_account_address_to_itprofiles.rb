# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AddAccountAddressToItprofiles < ActiveRecord::Migration[7.0]
  def change
    add_column :it1_profiles, :account_address, :string, index: { unique: true }
    add_column :it2_profiles, :account_address, :string, index: { unique: true }
  end
end
