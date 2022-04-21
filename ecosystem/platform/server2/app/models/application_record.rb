# frozen_string_literal: true

class ApplicationRecord < ActiveRecord::Base
  self.abstract_class = true

  # ensure hex is blank, or length 66 + 0x{hex}
  def self.validate_hex(field_name, allow_nil: true)
    validates field_name, length: { is: 6 }, format: { with: /\A0x(?:[A-F0-9]{2}){32}\z/i }, allow_nil:
  end
end
