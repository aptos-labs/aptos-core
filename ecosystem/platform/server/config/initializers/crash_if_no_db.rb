# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# This crashes if there is no DB present, for @Christian
# Only if this is running as a rails server
unless ENV.fetch('SKIP_DB_CHECK', nil)
  Rails.application.config.after_initialize do
    User.where(id: 1).exists?
  end
end
