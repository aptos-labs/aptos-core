# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

FactoryBot.define do
  factory :article do
    status { 'draft' }
    title { Faker::Book.title }
    content { Faker::Lorem.paragraphs(number: 3).map { |p| "<p>#{p}</p>" }.join }
    slug { Faker::Internet.slug }
  end
end
