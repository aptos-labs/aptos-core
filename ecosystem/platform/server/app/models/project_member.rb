# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ProjectMember < ApplicationRecord
  belongs_to :project
  belongs_to :user

  validates :project, presence: true
  validates :user, presence: true
  validates :role, presence: true, inclusion: { in: %w[member admin] }
  validates :public, presence: true
end
