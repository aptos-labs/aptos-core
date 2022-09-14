# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AddOwnerAddressToItprofiles < ActiveRecord::Migration[7.0]
  def change
    add_column :it3_profiles, :owner_address, :string, index: { unique: true }
  end
end
