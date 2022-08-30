# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

FactoryBot.define do
  factory :it3_profile do
    user { build :user }
    owner_key { "0x#{Faker::Crypto.sha256}" }
    consensus_key { "0x#{Faker::Crypto.sha256}#{Faker::Crypto.sha256}"[0...98] }
    consensus_pop { "0x#{Faker::Crypto.sha256}#{Faker::Crypto.sha256}#{Faker::Crypto.sha256}"[0...194] }
    account_key { "0x#{Faker::Crypto.sha256}" }
    network_key { "0x#{Faker::Crypto.sha256}" }
    validator_address { "0x#{Faker::Crypto.sha256}" }
    validator_port { 6180 }
    validator_api_port { 8080 }
    validator_metrics_port { 9101 }
    terms_accepted { true }
    fullnode_address { nil }
    fullnode_port { nil }
    fullnode_network_key { nil }
  end
end
