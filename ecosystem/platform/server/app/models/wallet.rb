# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class Wallet < ApplicationRecord
  VALID_NETWORKS = %w[ait3].freeze

  belongs_to :user

  validates :network, presence: true, inclusion: { in: VALID_NETWORKS }
  validates :public_key, presence: true, uniqueness: { scope: :network }, format: { with: /\A0x[a-f0-9]{64}\z/ }
end
