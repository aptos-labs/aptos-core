# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

FactoryBot.define do
  factory :it3_survey do
    user { build :user }
    persona { 'Node Operator' }
    participate_reason { Faker::Quote.yoda }
    qualified_reason { Faker::Quote.yoda }
    website { Faker::Internet.url }
    interest_reason { Faker::Quote.yoda }
  end
end
