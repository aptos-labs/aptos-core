# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

FactoryBot.define do
  factory :project_category do
    project { nil }
    category { Category.count > 0 ? Category.order('RANDOM()').first : build(:category) }
  end
end
