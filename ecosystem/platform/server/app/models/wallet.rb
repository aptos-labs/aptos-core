# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class Wallet < ApplicationRecord
  VALID_NETWORKS = %w[devnet testnet ait3].freeze
  VALID_WALLET_NAMES = %w[petra martian].freeze

  attr_accessor :challenge, :signed_challenge

  belongs_to :user

  validates :network, presence: true, inclusion: { in: VALID_NETWORKS }
  validates :wallet_name, presence: true, inclusion: { in: VALID_WALLET_NAMES }
  validates :public_key, presence: true, uniqueness: { scope: %i[network wallet_name] },
                         format: { with: /\A0x[a-f0-9]{64}\z/ }

  validates :challenge, presence: true, format: { with: /\A[0-9]{24}\z/ }
  validates :signed_challenge, presence: true, format: { with: /\A0x[a-f0-9]{128}\z/ }

  before_save :set_address

  def public_key_bytes
    [public_key[2..]].pack('H*')
  end

  def signed_challenge_bytes
    [signed_challenge[2..]].pack('H*')
  end

  def api_url
    case network
    when 'devnet'
      'https://fullnode.devnet.aptoslabs.com/v1'
    when 'testnet'
      'https://testnet.aptoslabs.com/v1'
    when 'ait3'
      'https://ait3.aptosdev.com/v1'
    else
      raise "API not mapped for #{network}!"
    end
  end

  private

  def set_address
    self.address = self.class.address_from_key(public_key)
  end
end
