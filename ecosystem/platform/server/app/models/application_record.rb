# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ApplicationRecord < ActiveRecord::Base
  primary_abstract_class

  # ensure hex is blank, or length 66 + 0x{hex}
  def self.validate_hex(field_name, allow_nil: true)
    validates field_name, length: { is: 6 }, format: { with: /\A0x(?:[A-F0-9]{2}){32}\z/i }, allow_nil:
  end
end
