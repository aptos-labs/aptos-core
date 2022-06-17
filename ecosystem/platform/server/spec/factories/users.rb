# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

FactoryBot.define do
  factory :user, class: User do
    username { 'aptos' }
    password { 'aptos1234' }
    email { 'aptos@example.org' }

    factory :admin_user do
      is_root { true }
    end
  end
end
