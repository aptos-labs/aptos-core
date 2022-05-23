# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class DividerComponent < ViewComponent::Base
  SCHEME_CLASSES = {
    primary: 'w-full flex text-center',
    secondary: 'w-full flex'
  }.freeze

  def initialize(scheme: :secondary,
                 **rest)
    @scheme = scheme
    @rest = rest
    @rest[:class] = [
      SCHEME_CLASSES[@scheme],
      @rest[:class]
    ]
  end
end
