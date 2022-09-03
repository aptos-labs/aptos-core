# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class Wallet < ApplicationRecord
  VALID_NETWORKS = %w[devnet ait3].freeze
  VALID_WALLET_NAMES = %w[petra].freeze

  belongs_to :user

  validates :network, presence: true, inclusion: { in: VALID_NETWORKS }
  validates :wallet_name, presence: true, inclusion: { in: VALID_WALLET_NAMES }
  validates :public_key, presence: true, uniqueness: { scope: :network }, format: { with: /\A0x[a-f0-9]{64}\z/ }

  before_save :set_address

  private

  def set_address
    self.address = self.class.address_from_key(public_key)
  end
end
