# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
# frozen_string_literal: true

class TableRowColumnComponent < ViewComponent::Base
  def initialize(**rest)
    rest[:class] = [
      'py-4 pr-16 first-of-type:pl-8 last-of-type:pr-8 text-neutral-100 ' \
      'first-of-type:rounded-l-lg last-of-type:rounded-r-lg',
      rest[:class]
    ]
    @rest = rest
  end

  def call
    content_tag :td, content, **@rest
  end
end
