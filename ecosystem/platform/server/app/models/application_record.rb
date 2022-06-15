# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'sha3'

class ApplicationRecord < ActiveRecord::Base
  primary_abstract_class

  def self.validate_aptos_address(field_name, allow_nil: true)
    validates field_name, format: { with: /\A0x[a-f0-9]{1,32}\z/i }, allow_nil:
  end

  # EX:
  # pubkey: 0x239F33C29C2FAE9E6094471D89857932FF3AFF97647F34BED6B48B5B6E20BB09
  # address: 0x2666d0ddd932a050a218d2050d4a840a023387f5ce9e97d41d5e484728121381
  # @param [String] pub_key
  # @return [String]
  def self.address_from_key(pub_key)
    bin = [pub_key.delete_prefix('0x')].pack('H*').unpack1('a*')
    "0x#{SHA3::Digest::SHA256.hexdigest(bin + 0.chr)}"
  end
end
