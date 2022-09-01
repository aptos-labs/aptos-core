# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

FactoryBot.define do
  factory :wallet do
    user
    network { 'ait3' }
    public_key { "0x#{Faker::Crypto.sha256}" }
    verified { false }
  end
end
