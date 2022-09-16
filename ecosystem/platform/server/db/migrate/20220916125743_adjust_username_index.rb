# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AdjustUsernameIndex < ActiveRecord::Migration[7.0]
  def change
    add_index :users, 'lower(username)', name: 'index_users_on_lower_username', unique: true
    remove_index :users, column: :username
  end
end
