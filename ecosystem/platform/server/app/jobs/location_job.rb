# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class LocationFetchError < StandardError; end

class LocationJob < ApplicationJob
  # Ex args: { it3_profile_id: 32 }
  def perform(args)
    it3_profile = It3Profile.find(args[:it3_profile_id])
    sentry_scope.set_user(id: it3_profile.user_id)
    sentry_scope.set_context(:job_info, { validator_address: it3_profile.validator_address })

    # pass zeroes as a hack here: we only need the validator address
    ip_resolver = NodeHelper::IPResolver.new(it3_profile.validator_address)

    unless ip_resolver.ip.ok
      raise LocationFetchError,
            "Error fetching IP for #{it3_profile.validator_address}: #{ip_resolver.ip.message}"
    end

    new_ip = ip_resolver.ip.ip.to_s
    if new_ip != it3_profile.validator_ip
      Rails.logger.warn "IP does not match one in profile for it3_profile ##{it3_profile.id} - " \
                        "#{it3_profile.validator_address}: was #{it3_profile.validator_ip}, got #{new_ip}"
    end

    location_res = ip_resolver.location
    unless location_res.ok
      # TODO: DO SOMETHING (SENTRY? THROW?) IF THIS IS NOT OK
      raise LocationFetchError, "Error fetching location for '#{it3_profile.validator_ip}': #{location_res.message}"
    end

    Location.upsert_from_maxmind!(it3_profile, location_res.record)
  end
end
