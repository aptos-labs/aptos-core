# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class CardOutlineComponent < ViewComponent::Base
  SCHEME_CLASSES = {
    hollow: 'self-start relative z-0 mix-blend-lighten pl-2 pb-2',
    filled: 'self-start relative z-0 pl-2 pb-2'
  }.freeze

  def initialize(scheme: :hollow,
                 **rest)
    @scheme = scheme
    @rest = rest
    @rest[:class] = [
      SCHEME_CLASSES[@scheme],
      @rest[:class]
    ]
  end
end
