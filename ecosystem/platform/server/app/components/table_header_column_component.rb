# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class TableHeaderColumnComponent < ViewComponent::Base
  include ApplicationHelper

  renders_one :tooltip, IconTooltipComponent

  def initialize(id = nil, title = nil, **rest)
    if id.is_a? String
      @id = nil
      @title = id
    else
      @id = id
      @title = title
    end

    @rest = rest
    @rest[:class] = [
      'py-4 pr-8 pl-2 first:rounded-l-lg last:rounded-r-lg uppercase text-base font-bold whitespace-nowrap',
      @rest[:class]
    ]
  end

  private

  def sort_direction
    sort = parse_sort(request.params).find do |key, _direction|
      key == @id.to_s
    end
    sort ? sort[1] : nil
  end

  def sort_arrow
    return if sort_direction.nil?

    if sort_direction.positive?
      '↑'
    else
      '↓'
    end
  end

  def href
    query = if sort_direction&.positive?
              "sort=-#{@id}"
            else
              "sort=#{@id}"
            end
    uri = URI::HTTP.build(path: request.path, query:)
    uri.request_uri
  end
end
