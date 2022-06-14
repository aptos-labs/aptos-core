# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# An offer for a user to claim an NFT (e.g. for promotional purposes).
class NftOffer < ApplicationRecord
  has_many :nfts

  validates :name, presence: true, uniqueness: true, format: { with: /\A[a-z_]+\z/ }
end
