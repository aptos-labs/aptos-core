# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# An offer for a user to claim an NFT (e.g. for promotional purposes).
class NftOffer
  include ActiveModel::Model

  attr_accessor :slug, :network

  def persisted?
    true
  end

  def to_key
    [slug]
  end
end
