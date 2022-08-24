# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

module SettingsHelper
  SidebarItem = Struct.new(:url, :name)

  def sidebar_items
    [
      SidebarItem.new(settings_profile_path, 'Profile'),
      SidebarItem.new(settings_notifications_path, 'Notifications'),
      SidebarItem.new(settings_connections_path, 'Connections')
    ]
  end
end
