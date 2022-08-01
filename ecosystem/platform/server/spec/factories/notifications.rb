# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

FactoryBot.define do
  factory :notification do
    recipient { nil }
    type { '' }
    params { '' }
    read_at { '2022-08-01 13:09:54' }
  end
end
