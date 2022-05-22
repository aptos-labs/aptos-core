# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'yaml'
require 'zip'

module GenesisHelper
  class Creator
    # Expected individual yaml format (file named after account address):
    #   account_address: 13bf4e245a0b76d445aa76ad39ff01c02d38621033aa0ddb90aeb8d723b3158e
    #   consensus_key: "0x9aa2353d22b30803f04a48b77369df9b7d56f7b2f2d001858f46f04e61f80f2e"
    #   account_key: "0x5d20fed59f31e1ef92696bd29b6b3b3ff3b580764f4577704bd2d7c1f3893092"
    #   validator_network_key: "0xa5e0636daf3e849cb72dbddf3770a6dafa92e646609f44e04750dbc6b225fb3d"
    #   validator_host:
    #     host: 166.88.163.219
    #     port: 16343
    #   full_node_network_key: "0xe268b86cc8cf9de341b17a018d510880467e8e9851811364289c70d665cc6d0f"
    #   full_node_host:
    #     host: 166.88.163.219
    #     port: 16342
    #   stake_amount: 1

    # @param [Array<Integer>] user_ids
    # @return [GenesisHelper::Creator]
    def self.from_user_ids(user_ids)
      new(It1Profile.where(user_id: user_ids))
    end

    # @param [It1Profile] it1_profile
    def self.convert_profile_to_obj(it1_profile)
      data = {
        account_address: it1_profile.account_address.delete_prefix('0x'),
        consensus_key: it1_profile.consensus_key,
        account_key: it1_profile.account_key,
        validator_network_key: it1_profile.network_key,
        validator_host: {
          host: it1_profile.validator_address,
          port: it1_profile.validator_port
        },
        stake_amount: 1
      }
      if it1_profile.fullnode_network_key.present?
        data[:full_node_network_key] = it1_profile.fullnode_network_key
        data[:full_node_host] = {
          host: it1_profile.fullnode_address,
          port: it1_profile.fullnode_port
        }
      else
        # Fallback
        data[:full_node_network_key] = it1_profile.network_key
      end
      data
    end

    def self.convert_profile_to_yaml(it1_profile)
      convert_profile_to_obj(it1_profile).deep_stringify_keys.to_yaml
    end

    # @param [Array<It1Profile>] it1_profiles
    def initialize(it1_profiles)
      @it1_profiles = it1_profiles
      @yamls = {}
      @it1_profiles.each do |it1_profile|
        @yamls[it1_profile.account_address] = self.class.convert_profile_to_yaml it1_profile
      end
    end

    def write_zip(zipfile_path)
      Zip::File.open(zipfile_path, create: true) do |zipfile|
        @yamls.each do |k, v|
          zipfile.get_output_stream("#{k}.yaml") { |f| f.write v }
        end
        zipfile.get_output_stream('layout.yaml') { |f| f.write layout_yaml }
      end
    end

    # layout.yaml format (each line is an address):
    #   root_key: "0x5243ca72b0766d9e9cbf2debf6153443b01a1e0e6d086c7ea206eaf6f8043956"
    #   users:
    #     - 13bf4e245a0b76d445aa76ad39ff01c02d38621033aa0ddb90aeb8d723b3158e
    #     - msmouse
    #     - greg
    #     - david_is_the_best
    #     - zk
    #     - rustie-validator
    #   chain_id: 10
    def layout_yaml
      {
        root_key: '0x5243ca72b0766d9e9cbf2debf6153443b01a1e0e6d086c7ea206eaf6f8043956',
        users: @yamls.keys,
        chain_id: 10
      }.deep_stringify_keys.to_yaml
    end
  end
end
