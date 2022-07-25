# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'set'
require 'resolv'

class It2Profile < ApplicationRecord
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
                                     validator_port validator_metrics_port validator_ip]

  def account_address
    self.class.address_from_key account_key
  end

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

  def nhc_job_running?
    nhc_job_id.present?
  end

  def enqueue_nhc_job(do_location)
    return unless id.present?

    if nhc_job_running?
      errors.add :base, 'Node Health Checker Job already enqueued'
      return
    end

    job = NhcJob.perform_later({ it2_profile_id: id, do_location: })
    self.nhc_job_id = "#{job.job_id&.presence || 'job-id'}|#{Time.now.utc}"
    self.nhc_output = nil
    update_columns(nhc_job_id:, nhc_output: nil)
  end

  def fullnode_network_key=(value)
    value = nil if value.blank?
    super(value)
  end

  def maybe_set_validated_to_false
    self.validator_verified = false if needs_revalidation?
  end

  private

  def check_validator_ipv4
    # If the updates don't require revalidation, don't do it
    return unless validator_ip_changed?

    return if validator_ip.blank? || validator_ip =~ Resolv::IPv4::Regex

    errors.add :validator_address, 'Address must resolve to or be an IPv4'
  end
end
