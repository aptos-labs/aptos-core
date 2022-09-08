# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ChangeNotificationPreferencesDefaultToFalse < ActiveRecord::Migration[7.0]
  def change
    change_column_default :notification_preferences, :governance_proposal_notification, from: true, to: false
    change_column_default :notification_preferences, :node_upgrade_notification, from: true, to: false
  end
end
