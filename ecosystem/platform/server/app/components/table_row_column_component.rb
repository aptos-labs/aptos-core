# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
# frozen_string_literal: true

class TableRowColumnComponent < ViewComponent::Base
  def initialize(**rest)
    rest[:class] = [
      'py-4 pr-8 pl-2 font-light border-b-2 border-dotted border-neutral-700',
      rest[:class]
    ]
    @rest = rest
  end

  def call
    content_tag :td, content, **@rest
  end
end
