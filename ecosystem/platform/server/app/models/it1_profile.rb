# frozen_string_literal: true

class It1Profile < ApplicationRecord
  belongs_to :user

  validates :consensus_key, presence: true, uniqueness: true, format: { with: /\A0x[a-f0-9]{64}\z/i }
  validates :account_key, presence: true, uniqueness: true, format: { with: /\A0x[a-f0-9]{64}\z/i }
  validates :network_key, presence: true, uniqueness: true, format: { with: /\A0x[a-f0-9]{64}\z/i }

  validates :validator_address, presence: true
  validates :validator_port, presence: true, numericality: { only_integer: true }
  validates :metrics_port, presence: true, numericality: { only_integer: true }

  validates :fullnode_port, numericality: { only_integer: true }, allow_nil: true

  validates :terms, acceptance: true
end
