# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class DialogComponent < ViewComponent::Base
  attr_reader :id

  def initialize(**rest)
    rest[:class] = [
      "rounded-xl border-2 border-teal-400",
      rest[:class],
    ]

    @id = rest[:id] || Random.uuid
    rest[:id] = @id

    rest[:data] ||= {}
    rest[:data][:controller] = 'dialog'

    @rest = rest
    @tag = :dialog
  end
end
