# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ProjectMilestone < ApplicationRecord
  belongs_to :project

  validates :project, presence: true
  validates :title, length: { maximum: 140 }
end
