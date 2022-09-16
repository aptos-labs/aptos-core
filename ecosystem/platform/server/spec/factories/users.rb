# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

FactoryBot.define do
  factory :user, class: User do
    username { Faker::Internet.username(specifier: 3..14, separators: %w[- _]) + 5.times.map { rand(10) }.join }
    password { Faker::Internet.password }
    email { Faker::Internet.email }
    bio { Faker::Lorem.paragraph }
    confirmed_at { Date.new }

    factory :admin_user do
      is_root { true }
    end
  end
end
