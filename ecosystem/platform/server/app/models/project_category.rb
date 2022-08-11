# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ProjectCategory < ApplicationRecord
  belongs_to :project
  belongs_to :category

  validates :project, presence: true
  validates :category, presence: true
end
