# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class CardOutlineComponent < ViewComponent::Base
  def initialize(**rest)
    @rest = rest
    @rest[:class] = [
      'self-start relative z-0 mix-blend-lighten pl-2 pb-2',
      @rest[:class]
    ]
  end
end
