# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

module UsersHelper
  NavItem = Struct.new(:url, :name)

  def nav_items(user)
    [
      NavItem.new(user_url(user), 'Overview'),
      NavItem.new(user_projects_url(user), 'Projects'),
      NavItem.new(user_activity_url(user), 'Activity'),
      NavItem.new(user_rewards_url(user), 'Rewards')
    ]
  end
end
