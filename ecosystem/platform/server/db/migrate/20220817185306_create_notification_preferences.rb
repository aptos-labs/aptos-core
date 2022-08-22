# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class CreateNotificationPreferences < ActiveRecord::Migration[7.0]
  def change
    create_table :notification_preferences do |t|
      t.references :user, null: false, foreign_key: true
      t.integer :delivery_method, null: false, default: 0
      t.boolean :node_upgrade_notification, null: false, default: true
      t.boolean :governance_proposal_notification, null: false, default: true

      t.timestamps
    end

    add_index :notification_preferences, %i[user_id delivery_method], unique: true
  end
end
