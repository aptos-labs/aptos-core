# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ApplicationRecord < ActiveRecord::Base
  primary_abstract_class

  def self.validate_aptos_address(field_name, allow_nil: true)
    validates field_name, format: { with: /\A0x[a-f0-9]{1,32}\z/i }, allow_nil:
  end
end
