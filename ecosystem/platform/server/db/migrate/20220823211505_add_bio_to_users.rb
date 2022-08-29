# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AddBioToUsers < ActiveRecord::Migration[7.0]
  def change
    add_column :users, :bio, :string
  end
end
