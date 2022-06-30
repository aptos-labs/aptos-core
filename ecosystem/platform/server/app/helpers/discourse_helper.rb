# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

module DiscourseHelper
  def self.base_url
    ENV.fetch('DISCOURSE_URL_BASE', 'https://forum.aptoslabs.com')
  end

  def self.discourse_url(path)
    URI.join(base_url, path)
  end
end
