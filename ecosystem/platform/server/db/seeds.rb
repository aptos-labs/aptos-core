# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# This file should contain all the record creation needed to seed the database with its default values.
# The data can then be loaded with the bin/rails db:seed command (or created alongside the database with db:setup).
#
# Examples:
#
#   movies = Movie.create([{ name: "Star Wars" }, { name: "Lord of the Rings" }])
#   Character.create(name: "Luke", movie: movies.first)

if Rails.env.development?
  User.create!(username: 'aptos', password: 'aptos1', password_confirmation: 'aptos1', is_root: true,
               confirmed_at: Time.now)
end
