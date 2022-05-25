# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class HeaderComponent < ViewComponent::Base
  def initialize(**rest)
    @rest = rest
    @rest[:class] = [
      'bg-black text-white flex px-4 items-center justify-between sticky top-0 h-16 z-10',
      @rest[:class]
    ]
  end
end
