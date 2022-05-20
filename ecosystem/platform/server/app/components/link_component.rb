# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class LinkComponent < ViewComponent::Base
  def initialize(**rest)
    @rest = rest
    @rest[:class] = [
      'underline decoration-teal-400 decoration-1 underline-offset-4 hover:decoration-2',
      @rest[:class]
    ]
  end

  def call
    content_tag :a, content, **@rest
  end
end
