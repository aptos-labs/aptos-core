# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
# frozen_string_literal: true

class TableRowComponent < ViewComponent::Base
  renders_many :columns, TableRowColumnComponent

  def initialize(**rest)
    rest[:class] = [
      'hover:bg-neutral-800'
    ]
    @rest = rest
  end

  def call
    content_tag :tr, content, **@rest
  end
end
