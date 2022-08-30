# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

FactoryBot.define do
  factory :project_category do
    project { build(:project, project_categories: [instance]) }
    category { Category.order('RANDOM()').first || build(:category) }
  end
end
