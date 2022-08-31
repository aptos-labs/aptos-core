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
    fullnode_network_key { "0x#{Faker::Crypto.sha256}" }
    fullnode_address { "0x#{Faker::Crypto.sha256}" }
    fullnode_port { 6180 }
    fullnode_api_port { 8080 }
    fullnode_metrics_port { 9101 }
    terms_accepted { true }
  end
end
