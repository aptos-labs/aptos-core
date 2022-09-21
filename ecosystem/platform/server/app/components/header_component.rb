# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class HeaderComponent < ViewComponent::Base
  NavItem = Struct.new(:url, :name, :title)
  NavGroup = Struct.new(:item, :children)

  NAV_GROUPS = [
    NavGroup.new(
      NavItem.new('#', 'Community', 'Aptos Community'),
      [
        NavItem.new('/community', 'Aptos Community', 'Aptos Community'),
        NavItem.new('https://forum.aptoslabs.com/', 'Discussion Forum', 'Aptos Forum'),
        NavItem.new('/incentivized-testnet', 'Incentivized Testnet', 'Aptos Forum')
      ]
    ),
    NavGroup.new(
      NavItem.new('#', 'Developers', 'Aptos Developers'),
      [
        NavItem.new('/developers', 'Resources', 'Aptos Developers'),
        NavItem.new('https://aptos.dev/', 'Documentation', 'Aptos Documentation')
      ]
    ),
    NavGroup.new(
      NavItem.new('#', 'Network', 'Aptos Network'),
      [
        NavItem.new('https://explorer.aptoslabs.com/', 'Explorer', 'Aptos Explorer')
      ]
    ),
    NavGroup.new(
      NavItem.new('#', 'About', 'About Aptos'),
      [
        NavItem.new('/currents', 'Currents', 'Aptos Currents'),
        NavItem.new('/careers', 'Careers', 'Aptos Careers'),
        NavItem.new(
          'https://aptos.dev/aptos-white-paper/aptos-white-paper-index/',
          'Whitepaper', 'Aptos Whitepaper'
        )
      ]
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
      'bg-neutral-900/[.95] border-b border-neutral-800 text-white flex px-4 sm:px-6 items-center',
      'sticky top-0 z-50 flex-wrap gap-4 h-20 backdrop-blur-lg',
      @rest[:class]
    ]
    @rest[:data] ||= {}
    @rest[:data][:controller] = 'header'
    @rest[:data][:action] = 'resize@window->header#windowResize click@window->header#windowClick'
  end

  def nav_groups
    NAV_GROUPS
  end

  def user_nav_items
    USER_NAV_ITEMS
  end
end
