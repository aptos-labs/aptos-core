# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# This crashes if there is no DB present, for @Christian
# Only if this is running as a rails server
if !ENV.fetch('SKIP_DB_CHECK', nil) && (ActiveRecord::Base.connection.data_source_exists? :users)
  Rails.application.config.after_initialize do
    User.where(id: 1).exists?
  end
end
