# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class KYCCompleteJobError < StandardError
end

class KYCCompleteJob < ApplicationJob
  # Ex args: { user_id: 32, inquiry_id=inq_syMMVRdEz7fswAa2hi }
  def perform(args)
    user = User.find(args[:user_id])
    inquiry_id = args[:inquiry_id]

    client = PersonaHelper::PersonaClient.new

    inquiry = client.inquiry(inquiry_id)

    raise KYCCompleteJobError, "Could not get inquiry '#{inquiry_id}' for user ##{user.id}" unless inquiry.present?

    status = inquiry['data']&.[]('attributes')&.[]('status')
    raise KYCCompleteJobError, "Inquiry was not complete! Status: '#{status}'" unless status == 'completed'

    user.update(completed_persona_inquiry_id: inquiry_id, kyc_status: 'completed')
  end
end
