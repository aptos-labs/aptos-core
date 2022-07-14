# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class FooterComponent < ViewComponent::Base
  NavItem = Struct.new(:url, :name)

  NAV_ITEMS = [
    NavItem.new('https://aptoslabs.com/developers', 'Developers'),
    NavItem.new('https://aptoslabs.com/currents', 'Currents'),
    NavItem.new('https://aptoslabs.com/careers', 'Careers'),
    NavItem.new('https://www.linkedin.com/company/aptoslabs', 'Team'),
    NavItem.new('https://aptoslabs.com/privacy', 'Privacy'),
    NavItem.new('https://aptoslabs.com/terms', 'Terms')
  ].freeze

  def initialize(**rest)
    @rest = rest
    @rest[:class] = [
      'bg-black text-white',
      @rest[:class]
    ]
  end

  def nav_items
    NAV_ITEMS
  end
end
