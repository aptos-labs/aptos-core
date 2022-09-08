# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

module ApplicationHelper
  SORT_ORDER = { '+' => 1, '-' => -1 }.freeze
  EMPTY_SORT = [].freeze

  def parse_sort(params)
    sort_param = params.fetch(:sort, nil)
    return EMPTY_SORT unless sort_param.is_a? String

    sort_param.split(',').map do |attr|
      sort_sign = attr =~ /\A[+-]/ ? attr.slice!(0) : '+'
      [attr, SORT_ORDER[sort_sign]]
    end
  end

  def truncate_address(string, separator: '…')
    string.truncate(
      (4 * 2) + separator.size, omission: "#{separator}#{string.last(4)}"
    )
  end
end
