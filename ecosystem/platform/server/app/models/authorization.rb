# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class Authorization < ApplicationRecord
  belongs_to :user, optional: true

  validates_uniqueness_of :uid, scope: [:provider]

  PROVIDER_NAME_MAP = { github: 'GitHub', google: 'Google', discord: 'Discord' }.with_indifferent_access.freeze

  def display_provider
    PROVIDER_NAME_MAP[provider] || provider
  end

  def display_name
    "#{provider} [#{username || email || full_name || uid}]"
  end

  def display_shortname
    username || email
  end
end
