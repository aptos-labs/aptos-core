# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# frozen_string_literal: true

class GlobalAnnouncementComponent < ViewComponent::Base
  def initialize(**rest)
    @rest = rest
    @rest[:class] = [
      'justify-between px-4 sm:px-6 py-3 w-full bg-teal-400 hidden',
      @rest[:class]
    ]

    @id = @rest[:id]
    @rest[:id] = @id

    @rest[:data] ||= {}
    @rest[:data][:controller] = 'visibility'
    @rest[:data][:visibility_target] = 'hideable'
  end
end
