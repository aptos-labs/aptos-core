# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class Category < ApplicationRecord
  has_many :projects, through: :project_categories

  validates :title, presence: true, length: { maximum: 140 }
end
