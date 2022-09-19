# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# An offer for a user to claim an NFT (e.g. for promotional purposes).
class NftOffer
  include ActiveModel::Model

  attr_accessor :slug, :network, :module_address, :private_key, :distinct_images

  # Constants for slugs
  APTOS_ZERO = 'aptos-zero'

  ID_TO_SLUG = {
    0 => APTOS_ZERO
  }.freeze

  SLUG_TO_ID = ID_TO_SLUG.invert.freeze

  def self.find(id)
    find_by(slug: ID_TO_SLUG.fetch(id.to_i, ''))
  end

  def self.find_by(slug:)
    case slug
    when APTOS_ZERO
      NftOffer.new(
        slug: APTOS_ZERO,
        network: Rails.env.development? ? 'devnet' : 'testnet',
        module_address: ENV.fetch('APTOS_ZERO_NFT_MODULE_ADDRESS'),
        private_key: ENV.fetch('APTOS_ZERO_NFT_PRIVATE_KEY'),
        distinct_images: true
      )
    else
      raise ActiveRecord::RecordNotFound
    end
  end

  def id
    SLUG_TO_ID[slug]
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
