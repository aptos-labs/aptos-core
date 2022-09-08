# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

BetterHtml.configure do |config|
  # Ignore ERB files from gems or Rails itself.
  config.template_exclusion_filter = proc { |filename| !filename.start_with?(Rails.root.to_s) }
end
