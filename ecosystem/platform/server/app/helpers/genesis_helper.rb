# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'yaml'

module GenesisHelper
  class Creator

    # @param [Array<Integer>] user_ids
    # @return [GenesisHelper::Creator]
    def self.from_user_ids(user_ids)
      new(It2Profile.where(user_id: user_ids).includes(:user, :location))
    end

    # @param [It2Profile] it2_profile
    def self.convert_profile_to_obj(it2_profile)
      user = it2_profile.user
      location = it2_profile.location
      {
        username: user.username,
        external_id: user.external_id,
        location: location.slice(:continent_name, :country_name, :city_name, :time_zone),
        discord_name: user.authorizations.where(provider: :discord).first.username,
        account_address: it2_profile.account_address.delete_prefix('0x'),
        consensus_key: it2_profile.consensus_key,
        account_key: it2_profile.account_key,
        validator_host_org: location.organization,
        validator_network_key: it2_profile.network_key,
        validator_host: {
          host: it2_profile.validator_address,
          port: it2_profile.validator_port
        }
      }
    end

    # @param [Array<It2Profile>] it2_profiles
    def initialize(it2_profiles)
      @it2_profiles = it2_profiles
      @yamls = {}
      @it2_profiles.each do |it2_profile|
        @yamls[it2_profile.account_address] = self.class.convert_profile_to_obj it2_profile
      end
    end

  end
end
