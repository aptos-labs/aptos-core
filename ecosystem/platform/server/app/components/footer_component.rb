# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class FooterComponent < ViewComponent::Base
  def initialize(**rest)
    @rest = rest
    @rest[:class] = [
      'bg-black text-white flex p-6 sm:px-12 sm:py-8 items-center flex-col sm:flex-row gap-6 text-center md:text-left',
      @rest[:class]
    ]
  end
end
