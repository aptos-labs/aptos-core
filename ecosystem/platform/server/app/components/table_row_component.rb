# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
# frozen_string_literal: true

class TableRowComponent < ViewComponent::Base
  renders_many :columns, TableRowColumnComponent

  def initialize(**rest)
    @rest = rest
    rest[:class] = [
      'bg-neutral-800 hover:bg-neutral-800/50 text-sm',
      rest[:class]
    ]
    @rest[:data] ||= {}
    @rest[:data][:controller] = 'table_row'
    @rest[:data][:action] = 'click->table_row#tableRowClick'
  end

  def call
    content_tag :tr, content, **@rest
  end
end
