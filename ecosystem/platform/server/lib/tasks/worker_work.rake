# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'digest/sha1'

namespace :worker do
  desc 'Process delayed jobs for Aptos Cloud'
  task work: :environment do
    logger = ActiveSupport::Logger.new($stdout)
    logger.formatter = Rails.configuration.log_formatter
    Rails.configuration.logger = ActiveSupport::TaggedLogging.new(logger)
    Rails.configuration.log_level = :debug
    Delayed.logger = Rails.configuration.logger
    Rails.logger = Delayed.logger

    worker = Delayed::Worker.new
    worker.name_prefix = Digest::SHA1.hexdigest("#{rand(100_000_000)}--#{Time.now}")[0, 12]

    worker.name = begin
      "#{worker.name_prefix} host:#{Socket.gethostname} pid:#{Process.pid}"
    rescue StandardError
      "#{worker.name_prefix} pid:#{Process.pid}"
    end
    Rails.logger.info "STARTING WORKER #{worker.name}"

    worker.start
  end
end
