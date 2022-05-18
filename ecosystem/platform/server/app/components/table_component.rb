# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
# frozen_string_literal: true

class TableComponent < ViewComponent::Base
  renders_many :columns, TableHeaderColumnComponent
  renders_one :body

  def initialize(**rest)
    rest[:class] = [
      'font-mono',
      rest[:class]
    ]
    @rest = rest
  end
end
