# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

Sentry.init do |config|
  config.dsn = ENV.fetch('SENTRY_DSN', nil)
  config.breadcrumbs_logger = %i[active_support_logger http_logger]

  # To activate performance monitoring, set one of these options.
  # We recommend adjusting the value in production:
  config.traces_sample_rate = 0.8
  config.environment = Rails.env
end
