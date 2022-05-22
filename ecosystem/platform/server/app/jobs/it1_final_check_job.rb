# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class It1FinalCheckError < StandardError; end

class It1FinalCheckJob < ApplicationJob
  # Ex args: { it1_profile_id: 32 }
  def perform(args)
    it1_profile = It1Profile.find(args[:it1_profile_id])
    sentry_scope.set_user(id: it1_profile.user_id)
    sentry_scope.set_context(:job_info, { validator_address: it1_profile.validator_address })

    node_verifier = NodeHelper::NodeVerifier.new(it1_profile.validator_address, it1_profile.validator_metrics_port, 0)

    unless node_verifier.ip.ok
      it1_profile.update(validator_verified_final: false)
      raise It1FinalCheckError,
            "Error fetching IP for #{it1_profile.validator_address}: #{node_verifier.ip.message}"
    end

    res = node_verifier.fetch_json_metrics
    unless res.ok
      it1_profile.update(validator_verified_final: false)
      raise It1FinalCheckError, "Error fetching metrics json for '#{it1_profile.validator_ip}': #{res.message}"
    end

    it1_profile.update(validator_verified_final: true, metrics_data: res.data)
  end
end
