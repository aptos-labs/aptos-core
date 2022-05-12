# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AddGuidToUser < ActiveRecord::Migration[7.0]
  def change
    change_table :users do |t|
      t.uuid :external_id, null: false, default: 'gen_random_uuid()'
    end
  end
end
