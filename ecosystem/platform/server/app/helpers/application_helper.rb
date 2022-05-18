# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

module ApplicationHelper
  SORT_ORDER = { '+' => 1, '-' => -1 }.freeze

  def parse_sort(params)
    params.fetch(:sort, '').split(',').map do |attr|
      sort_sign = attr =~ /\A[+-]/ ? attr.slice!(0) : '+'
      [attr, SORT_ORDER[sort_sign]]
    end
  end
end
