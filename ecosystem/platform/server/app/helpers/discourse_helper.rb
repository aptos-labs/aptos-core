# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

module DiscourseHelper
  def self.base_url
    ENV.fetch('DISCOURSE_URL_BASE', 'https://forum.aptoslabs.com')
  end

  def self.sso_secret
    ENV.fetch('DISCOURSE_SECRET', nil)
  end

  # @return DiscourseApi::Client
  def self.system_client
    client = DiscourseApi::Client.new(DiscourseHelper.base_url)
    client.api_key = ENV.fetch('DISCOURSE_SYSTEM_API_KEY')
    client.api_username = ENV.fetch('DISCOURSE_SYSTEM_USERNAME')
    client
  end

  def self.discourse_url(path)
    URI.join(base_url, path)
  end
end
