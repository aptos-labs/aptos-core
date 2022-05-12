# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class KYCCompleteJobError < StandardError; end

class KYCCompleteJob < ApplicationJob
  # Ex args: { user_id: 32, inquiry_id=inq_syMMVRdEz7fswAa2hi }
  def perform(args)
    user = User.find(args[:user_id])
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
    raise KYCCompleteJobError, "Inquiry was not complete! Status: '#{status}'" unless status == 'completed'

    user.update(completed_persona_inquiry_id: inquiry_id, kyc_status: 'completed')
    user.maybe_send_ait1_registration_complete_email
  end
end
