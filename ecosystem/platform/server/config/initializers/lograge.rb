# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

Rails.application.configure do
  config.lograge.enabled = !Rails.env.development? || ENV.fetch('LOGRAGE_IN_DEVELOPMENT', nil) == 'true'
  config.lograge.formatter = Lograge::Formatters::Json.new

  config.lograge.custom_options = lambda do |event|
    result = {}
    result[:time] = Time.now.to_f
    result[:request_id] = event.payload[:request_id]
    result[:user_id] = event.payload[:user_id] if event.payload[:user_id].present?
    result
  end
end
