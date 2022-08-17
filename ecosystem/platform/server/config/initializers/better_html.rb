# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

BetterHtml.configure do |config|
  config.allow_single_quoted_attributes = true
  # Ignore ERB files from gems or Rails itself.
  config.template_exclusion_filter = proc { |filename| !filename.start_with?(Rails.root.to_s) }
end
