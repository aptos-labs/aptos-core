# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class LocationFetchError < StandardError; end

class LocationJob < ApplicationJob
  # Ex args: { it1_profile_id: 32 }
  def perform(args)
    it1_profile = It1Profile.find(args[:it1_profile_id])
    sentry_scope.set_user(id: it1_profile.user_id)
    sentry_scope.set_context(:job_info, { validator_address: it1_profile.validator_address })

    # pass zeroes as a hack here: we only need the validator address
    node_verifier = NodeHelper::NodeVerifier.new(it1_profile.validator_address, 0, 0)

    unless node_verifier.ip.ok
      raise LocationFetchError,
            "Error fetching IP for #{it1_profile.validator_address}: #{node_verifier.ip.message}"
    end

    new_ip = node_verifier.ip.ip
    if new_ip != it1_profile.validator_ip
      Rails.logger.warn "IP does not match one in profile for it1_profile ##{it1_profile.id} - "\
                        "#{it1_profile.validator_address}: was #{it1_profile.validator_ip}, got #{new_ip}"
    end

    location_res = node_verifier.location

    unless location_res.ok
      # TODO: DO SOMETHING (SENTRY? THROW?) IF THIS IS NOT OK
      raise LocationFetchError, "Error fetching location for '#{it1_profile.validator_ip}': #{location_res.message}"
    end

    Location.upsert_from_maxmind!(it1_profile, location_res.record)
  end
end
