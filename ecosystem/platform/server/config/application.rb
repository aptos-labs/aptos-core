# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require_relative 'boot'

require 'rails/all'

# Require the gems listed in Gemfile, including any gems
# you've limited to :test, :development, or :production.
Bundler.require(*Rails.groups)

module CommunityPlatform
  class Application < Rails::Application
    # Initialize configuration defaults for originally generated Rails version.
    config.load_defaults 7.0

    keyfile = Rails.root.join('aptos-community-sa-keys.json')
    if !Rails.env.test? && ENV['STORAGE_SERVICE_ACCOUNT_KEY'].present? && !keyfile.exist?
      File.write(keyfile, ENV.fetch('STORAGE_SERVICE_ACCOUNT_KEY'))
    end

    # Configuration for the application, engines, and railties goes here.
    #
    # These settings can be overridden in specific environments using the files
    # in config/environments, which are processed later.
    #
    # config.time_zone = "Central Time (US & Canada)"
    # config.eager_load_paths << Rails.root.join("extras")

    # config.debug_exception_response_format = :api

    # Enable gzip compression for HTTP responses.
    # TODO: Remove when the CDN handles compression.
    config.middleware.insert_before(Rack::Sendfile, Rack::Deflater)

    # View helpers should be scoped to the corresponding controller.
    config.action_controller.include_all_helpers = false

    # Image analysis is not needed.
    config.active_storage.analyzers = []
  end
end
