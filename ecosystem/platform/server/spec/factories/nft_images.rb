# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

FactoryBot.define do
  factory :nft_image do
    slug { 'aptos-zero' }
    image_number { Faker::Number.number(digits: 3) }
  end
end
