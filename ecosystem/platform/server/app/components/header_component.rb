# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class HeaderComponent < ViewComponent::Base
  NavItem = Struct.new(:url, :name, :title)
  NavGroup = Struct.new(:item, :children)

  NAV_GROUPS = [
    NavGroup.new(
      NavItem.new('/', 'Community', 'Aptos Community'),
      [
        NavItem.new('/it1', 'AIT1', 'Incentivized Testnet 1 Results'),
        NavItem.new('/it2', 'AIT2', 'Incentivized Testnet 2'),
        NavItem.new('https://forum.aptoslabs.com/', 'Forum', 'Aptos Forum')
      ]
    ),
    NavGroup.new(
      NavItem.new('https://aptoslabs.com/developers', 'Developers', 'Aptos Developers'),
      [
        NavItem.new('https://aptos.dev/', 'Documentation', 'Aptos Documentation')
      ]
    ),
    NavGroup.new(
      NavItem.new('#', 'Network', 'Aptos Network'),
      [
        NavItem.new('https://explorer.devnet.aptos.dev/', 'Explorer', 'Aptos Explorer'),
        NavItem.new('https://status.devnet.aptos.dev/', 'Network Status', 'Aptos Network Status')
      ]
    ),
    NavGroup.new(
      NavItem.new('#', 'About', 'About Aptos'),
      [
        NavItem.new('https://aptoslabs.com/careers', 'Careers', 'Aptos Careers')
      ]
    ),
    NavGroup.new(
      NavItem.new('https://aptoslabs.com/currents', 'Currents', 'Aptos Currents'),
      []
    )
  ].freeze

  USER_NAV_ITEMS = [
    NavItem.new('/settings', 'Settings', 'Account Settings'),
    NavItem.new('/users/sign_out', 'Sign Out', 'Sign Out')
  ].freeze

  def initialize(user: nil, **rest)
    @user = user
    @rest = rest
    @rest[:class] = [
      'bg-neutral-900 border-b border-black text-white flex px-4 sm:px-6 items-center sticky top-0 z-10',
      'flex-wrap gap-4',
      @rest[:class]
    ]
    @rest[:data] ||= {}
    @rest[:data][:controller] = 'header'
  end

  def nav_groups
    NAV_GROUPS
  end

  def user_nav_items
    USER_NAV_ITEMS
  end
end
