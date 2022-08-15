# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

FactoryBot.define do
  factory :project_member do
    project { nil }
    user { build :user }
    role { 'admin' }
    public { true }
  end
end
