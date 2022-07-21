# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class HeaderComponent < ViewComponent::Base
  NavItem = Struct.new(:url, :name, :title)
  NAV_ITEMS = [
    NavItem.new('/it1', 'AIT1', 'Incentivized Testnet 1 Results'),
    NavItem.new('/it2', 'AIT2', 'Incentivized Testnet 2'),
    NavItem.new('https://aptos.dev/', 'Docs', 'Aptos Docs'),
    NavItem.new('https://forum.aptoslabs.com/', 'Forum', 'Aptos Forum'),
    NavItem.new('https://explorer.devnet.aptos.dev/', 'Explorer', 'Aptos Explorer')
  ].freeze

  USER_NAV_ITEMS = [
    NavItem.new('/settings', 'Settings', 'Account Settings'),
    NavItem.new('/users/sign_out', 'Sign Out', 'Sign Out')
  ].freeze

  def initialize(user: nil, **rest)
    @user = user
    @rest = rest
    @rest[:class] = [
      'bg-black text-white flex px-4 sm:px-6 items-center sticky top-0 z-10',
      'flex-wrap gap-4',
      @rest[:class]
    ]
    @rest[:data] ||= {}
    @rest[:data][:controller] = 'header'
  end

  def nav_items
    NAV_ITEMS
  end

  def user_nav_items
    USER_NAV_ITEMS
  end
end
