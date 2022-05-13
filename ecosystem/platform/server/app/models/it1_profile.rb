# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
# frozen_string_literal: true

require 'set'
require 'resolv'

class It1Profile < ApplicationRecord
  belongs_to :user
  validates :user_id, uniqueness: true

  has_one :location, as: :item

  validates :consensus_key, presence: true, uniqueness: true, format: { with: /\A0x[a-f0-9]{64}\z/i }
  validates :account_key, presence: true, uniqueness: true, format: { with: /\A0x[a-f0-9]{64}\z/i }
  validates :network_key, presence: true, uniqueness: true, format: { with: /\A0x[a-f0-9]{64}\z/i }

  validates :validator_address, presence: true
  validates :validator_port, presence: true, numericality: { only_integer: true }
  validates :validator_api_port, presence: true, numericality: { only_integer: true }
  validates :validator_metrics_port, presence: true, numericality: { only_integer: true }

  validates :fullnode_port, numericality: { only_integer: true }, allow_nil: true
  validates :fullnode_network_key, uniqueness: true, format: { with: /\A0x[a-f0-9]{64}\z/i }, allow_nil: true,
                                   allow_blank: true

  validates :terms_accepted, acceptance: true

  validate :check_validator_ipv4

  before_save :maybe_set_validated_to_false

  CHANGES_TO_REVALIDATE = Set.new %w[consensus_key account_key network_key validator_address validator_api_port
                                     validator_metrics_port]

  def validator_port
    self[:validator_port] || 6180
  end

  def validator_api_port
    self[:validator_api_port] || 8080
  end

  def validator_metrics_port
    self[:validator_metrics_port] || 9101
  end

  def needs_revalidation?
    changed.map { |field| CHANGES_TO_REVALIDATE.include? field }.any?
  end

  private

  def check_validator_ipv4
    # If the updates don't require revalidation, don't do it
    return unless validator_ip_changed?

    return if validator_ip.blank? || validator_ip =~ Resolv::IPv4::Regex

    errors.add :validator_address, 'Address must resolve to or be an IPv4'
  end

  def maybe_set_validated_to_false
    self.validator_verified = false if needs_revalidation?
  end
end
