# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

FactoryBot.define do
  factory :project_screenshot do
    project { nil }
    url { Faker::LoremFlickr.image(size: '1920x1080') }
  end
end
