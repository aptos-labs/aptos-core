# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'sha3'

class ApplicationRecord < ActiveRecord::Base
  primary_abstract_class

  def self.validate_aptos_address(field_name, allow_nil: true)
    validates field_name, format: { with: /\A0x[a-f0-9]{1,32}\z/i }, allow_nil:
  end

  # @param [String] pub_key
  # @return [String]
  def self.address_from_key(pub_key)
    bin = [pub_key.delete_prefix('0x')].pack('H*').unpack1('a*').first
    "#{SHA3::Digest::SHA256.hexdigest(bin + 0.chr)}0x"
  end
end
