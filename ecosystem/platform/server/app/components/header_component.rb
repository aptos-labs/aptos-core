# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class HeaderComponent < ViewComponent::Base
  NavItem = Struct.new(:url, :name, :title)
  NAV_ITEMS = [
    NavItem.new('/it1', 'AIT1', 'Incentivized Testnet 1 Results'),
    NavItem.new('/it2', 'AIT2', 'Incentivized Testnet 2'),
    NavItem.new('https://aptos.dev/', 'Docs', 'Aptos Docs'),
    NavItem.new('https://explorer.devnet.aptos.dev/', 'Explorer', 'Aptos Explorer'),
    NavItem.new(DiscourseHelper.base_url, 'Forum', 'Community Forum'),
    NavItem.new('/settings', 'Settings', 'Settings')
  ].freeze

  def initialize(**rest)
    @rest = rest
    @rest[:class] = [
      'bg-black text-white flex px-4 sm:px-6 items-center justify-between sticky top-0 z-10',
      'flex-wrap',
      @rest[:class]
    ]
    @rest[:data] ||= {}
    @rest[:data][:controller] = 'header'
  end

  def nav_items
    NAV_ITEMS
  end
end
