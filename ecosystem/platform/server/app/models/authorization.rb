# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class Authorization < ApplicationRecord
  belongs_to :user, optional: true

  validates_uniqueness_of :uid, scope: [:provider]

  def display_name
    "#{provider} [#{full_name || username || email || uid}]"
  end
end
