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

    node_verifier = NodeHelper::NodeVerifier.new(it2_profile.validator_address, it2_profile.validator_metrics_port, 0)

    unless node_verifier.ip.ok
      it2_profile.update(validator_verified_final: false)
      raise It2FinalCheckError,
            "Error fetching IP for #{it2_profile.validator_address}: #{node_verifier.ip.message}"
    end

    res = node_verifier.fetch_json_metrics
    unless res.ok
      it2_profile.update(validator_verified_final: false)
      raise It2FinalCheckError, "Error fetching metrics json for '#{it2_profile.validator_ip}': #{res.message}"
    end

    it2_profile.update(validator_verified_final: true, metrics_data: res.data)
  end
end
