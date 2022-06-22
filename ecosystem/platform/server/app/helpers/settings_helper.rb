# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

module SettingsHelper
  SidebarItem = Struct.new(:url, :name)

  def sidebar_items
    [
      SidebarItem.new(settings_profile_url, 'Profile'),
      SidebarItem.new(settings_connections_url, 'Connections')
    ]
  end
end
