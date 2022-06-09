# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class LeaderboardController < ApplicationController
  IT1_METRIC_KEYS = %i[rank validator liveness participation latest_reported_timestamp].freeze
  IT1_RESULTS = File.read(File.join(Rails.root, 'public/it1_leaderboard_final.json'))
  It1Metric = Struct.new(*IT1_METRIC_KEYS)

  def it1
    expires_in 1.minute, public: true
    default_sort = [[:participation, -1], [:liveness, -1], [:latest_reported_timestamp, -1]]
    @metrics, @last_updated = Rails.cache.fetch(:it1_leaderboard, expires_in: 1.minute) do
      metrics = JSON.parse(IT1_RESULTS).map do |metric|
        timestamp = metric['latest_reported_timestamp']&.blank? ? nil : DateTime.parse(metric['latest_reported_timestamp']).to_f
        It1Metric.new(
          -1,
          metric['validator'],
          metric['liveness'].to_f,
          metric['participation'].to_f,
          timestamp
        )
      end
      sort_metrics!(metrics, default_sort)
      metrics.each_with_index do |metric, i|
        metric.rank = i + 1
      end
      [metrics, Time.now]
    end

    @sort_columns = %w[rank liveness participation latest_reported_timestamp]
    sort = sort_params(@sort_columns)
    sort_metrics!(@metrics, sort) if sort
  end

  private

  def sort_params(valid_columns)
    helpers.parse_sort(params).filter_map do |key, direction|
      [key.to_sym, direction] if valid_columns.include? key
    end
  end

  def sort_metrics!(metrics, sort)
    metrics.sort_by! do |metric|
      sort.map do |key, direction|
        value = metric[key] || -Float::INFINITY
        value * direction
      end
    end
  end
end
