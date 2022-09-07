# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# An offer for a user to claim an NFT (e.g. for promotional purposes).
class NftOffer
  include ActiveModel::Model

  attr_accessor :slug, :network, :module_address, :private_key, :distinct_images

  # Constants for slugs
  APTOS_ZERO = 'aptos-zero'

  def self.find(slug)
    case slug
    when APTOS_ZERO
      NftOffer.new(
        slug: APTOS_ZERO,
        network: 'testnet',
        module_address: ENV.fetch('APTOS_ZERO_NFT_MODULE_ADDRESS'),
        private_key: ENV.fetch('APTOS_ZERO_NFT_PRIVATE_KEY'),
        distinct_images: true
      )
    else
      raise ActiveRecord::RecordNotFound
    end
  end

  def private_key_bytes
    [private_key[2..]].pack('H*')
  end

  def persisted?
    true
  end

  def to_key
    [slug]
  end
end
