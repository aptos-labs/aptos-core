# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class NotificationPreference < ApplicationRecord
  belongs_to :user

  enum delivery_method: {
    database: 0,
    email: 1
  }, _prefix: true

  validates :user, uniqueness: { scope: :delivery_method }
end
