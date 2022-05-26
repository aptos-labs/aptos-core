# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class DialogComponent < ViewComponent::Base
  attr_reader :id

  renders_one :title
  renders_one :body

  def initialize(**rest)
    @rest = rest
    @rest[:class] = [
      'rounded-xl border-none bg-neutral-900 text-white p-8 w-96',
      @rest[:class]
    ]

    @id = @rest[:id] || Random.uuid
    @rest[:id] = @id

    @rest[:data] ||= {}
    @rest[:data][:controller] = 'dialog'
  end
end
