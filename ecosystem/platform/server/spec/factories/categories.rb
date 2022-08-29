# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

FactoryBot.define do
  factory :category do
    title { %w[NFTs DeFi Gaming Tooling Wallets Data Lending Other].sample }
  end
end
