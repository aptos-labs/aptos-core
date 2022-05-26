# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class FooterComponent < ViewComponent::Base
  def initialize(**rest)
    @rest = rest
    @rest[:class] = [
      'bg-black px-4 sm:px-6 py-8 text-white',
      @rest[:class]
    ]
  end
end
