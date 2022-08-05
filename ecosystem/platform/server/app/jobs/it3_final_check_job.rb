# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class It3FinalCheckError < StandardError; end

class It3FinalCheckJob < ApplicationJob
  # Ex args: { it3_profile_id: 32 }
  def perform(args)
    it3_profile = It3Profile.find(args[:it3_profile_id])
    sentry_scope.set_user(id: it3_profile.user_id)
    sentry_scope.set_context(:job_info, { validator_address: it3_profile.validator_address })

    node_verifier = NodeHelper::NodeVerifier.new(it3_profile.validator_address, it3_profile.validator_metrics_port, 0)

    unless node_verifier.ip.ok
      it3_profile.update(validator_verified_final: false)
      raise It3FinalCheckError,
            "Error fetching IP for #{it3_profile.validator_address}: #{node_verifier.ip.message}"
    end

    res = node_verifier.fetch_json_metrics
    unless res.ok
      it3_profile.update(validator_verified_final: false)
      raise It3FinalCheckError, "Error fetching metrics json for '#{it3_profile.validator_ip}': #{res.message}"
    end

    it3_profile.update(validator_verified_final: true, metrics_data: res.data)
  end
end
