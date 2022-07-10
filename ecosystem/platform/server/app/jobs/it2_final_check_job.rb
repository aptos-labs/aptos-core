# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class It2FinalCheckError < StandardError; end

class It2FinalCheckJob < ApplicationJob
  # Ex args: { it2_profile_id: 32 }
  def perform(args)
    it2_profile = It2Profile.find(args[:it2_profile_id])
    sentry_scope.set_user(id: it2_profile.user_id)
    sentry_scope.set_context(:job_info, { validator_address: it2_profile.validator_address })

    v = NodeHelper::IPResolver.new(it2_profile.validator_address)
    unless v.ip.ok
      it2_profile.update(validator_verified_final: false)
      raise It2FinalCheckError,
            "Error fetching IP for #{it2_profile.validator_address}: #{v.ip.message}"
    end

    results = NhcJob.perform_now({ it2_profile_id: it2_profile.id, do_location: false })
    it2_profile.update(metrics_data: results)
  end
end
