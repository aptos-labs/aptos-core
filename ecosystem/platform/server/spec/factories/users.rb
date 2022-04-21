# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

FactoryBot.define do
  factory :user, class: User do
    password { 'aptos' }

    factory :admin_user do
      is_root { true }
    end
  end
end
