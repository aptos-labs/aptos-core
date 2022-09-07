# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

FactoryBot.define do
  factory :wallet do
    user
    network { 'ait3' }
    wallet_name { 'petra' }
    public_key { "0x#{Faker::Crypto.sha256}" }
    challenge { '0' * 24 }
    signed_challenge { "0x#{Faker::Crypto.sha256}#{Faker::Crypto.sha256}" }
  end
end
