# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class FooterComponent < ViewComponent::Base
  NavItem = Struct.new(:url, :name)

  NAV_ITEMS = [
    NavItem.new('/developers', 'Developers'),
    NavItem.new('/currents', 'Currents'),
    NavItem.new('/careers', 'Careers'),
    NavItem.new('https://www.linkedin.com/company/aptoslabs', 'Team'),
    NavItem.new('https://aptos.dev/aptos-white-paper/aptos-white-paper-index/',
                'Whitepaper'),
    NavItem.new('/privacy', 'Privacy'),
    NavItem.new('/terms', 'Terms')
  ].freeze

  def initialize(**rest)
    @rest = rest
    @rest[:class] = [
      'bg-neutral-900 text-neutral-100',
      @rest[:class]
    ]
  end

  def nav_items
    NAV_ITEMS
  end
end
