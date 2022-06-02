# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AddIndexesToUsers < ActiveRecord::Migration[7.0]
  def change
    add_index :users, :external_id
    add_index :users, :current_sign_in_ip
    add_index :users, :last_sign_in_ip
  end
end
