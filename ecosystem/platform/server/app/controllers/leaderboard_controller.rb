# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class LeaderboardController < ApplicationController
  It1MetricKeys = %i[rank validator liveness participation latest_reported_timestamp].freeze
  It1Metric = Struct.new(*It1MetricKeys)

  def it1
    expires_in 1.minute, public: true
    default_sort = [[:participation, -1], [:liveness, -1]]
    @metrics = Rails.cache.fetch(:it1_leaderboard, expires_in: 1.minute) do
      response = HTTParty.get(ENV.fetch('LEADERBOARD_IT1_URL'))
      metrics = JSON.parse(response.body).map do |metric|
        It1Metric.new(
          -1,
          metric['validator'],
          metric['liveness'].to_f,
          metric['participation'].to_f,
          DateTime.parse(metric['latest_reported_timestamp']).to_f
        )
      end
      metrics.sort_by! do |metric|
        default_sort.map { |key, direction| metric[key] * direction }
      end
      metrics.each_with_index do |metric, i|
        metric.rank = i + 1
      end
      metrics
    end

    @sort_columns = %w[rank liveness participation latest_reported_timestamp]
    sort = sort_params(@sort_columns)
    if sort
      @metrics.sort_by! do |metric|
        sort.map { |key, direction| metric[key] * direction }
      end
    end
  end

  private

  def sort_params(valid_columns)
    helpers.parse_sort(params).filter_map do |key, direction|
      [key.to_sym, direction] if valid_columns.include? key
    end
  end
end
