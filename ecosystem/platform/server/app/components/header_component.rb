# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class HeaderComponent < ViewComponent::Base
  NavItem = Struct.new(:url, :name, :title)
  NAV_ITEMS = [
    NavItem.new('/it1', 'Validator Status', 'AIT1 Validator Status'),
    NavItem.new('https://aptos.dev/', 'Docs', 'Aptos Docs'),
    NavItem.new('https://explorer.devnet.aptos.dev/', 'Explorer', 'Aptos Explorer'),
    NavItem.new(discourse_forum_url, 'Forum', 'Community Forum')
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

  def discourse_forum_url
    if current_user&.registration_completed?
      DiscourseHelper.discourse_url('/session/sso?return_path=%2F')
    else
      DiscourseHelper.base_url
    end
  end
end
