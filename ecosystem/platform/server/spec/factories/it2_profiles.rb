# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

FactoryBot.define do
  factory :it2_profile do
    user { nil }
    consensus_key { '0xbcaa0d44b821a745bc29767713cd78dbc88da73679e3ccdf5c145a2b4f7b17ac' }
    account_key { '0x7964a378e4c6d387d900c6e02430b7ee8263a977ace368484fc72c3b8469f520' }
    network_key { '0x2b0ebca9776bd79dcd3c0551e784965e87e8a1551d52c4a48758e1df2122064b' }
    validator_address { '127.0.0.1' }
    validator_port { 6180 }
    validator_metrics_port { 9101 }
    validator_api_port { 8080 }
    terms_accepted { true }
  end
end
