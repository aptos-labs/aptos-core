# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class KYCCompleteJobError < StandardError; end

VALID_STATUSES = %w[approved completed].freeze

class KYCCompleteJob < ApplicationJob
  # Ex args: { user_id: 32, inquiry_id=inq_syMMVRdEz7fswAa2hi, external_id: 141bc487-e025-418e-6e32-b7897060841c }
  def perform(args)
    user = if args[:user_id].present?
             User.find(args[:user_id])
           else
             User.where(external_id: args[:external_id]).first!
           end
    sentry_scope.set_user(id: user.id)

    inquiry_id = args[:inquiry_id]

    client = PersonaHelper::PersonaClient.new

    inquiry = client.inquiry(inquiry_id)

    raise KYCCompleteJobError, "Could not get inquiry '#{inquiry_id}' for user ##{user.id}" unless inquiry.present?

    reference_id = inquiry['data']&.[]('attributes')&.[]('reference_id')
    unless user.external_id == reference_id
      raise KYCCompleteJobError, "Inquiry '#{inquiry_id}' reference_id did not match expected user ##{user.id}"
    end

    status = inquiry['data']&.[]('attributes')&.[]('status')
    raise KYCCompleteJobError, "Inquiry was not complete! Status: '#{status}'" unless VALID_STATUSES.include? status

    user.update(completed_persona_inquiry_id: inquiry_id, kyc_status: 'completed')
    user.maybe_send_ait1_registration_complete_email
  end
end
