# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class It2Survey < ApplicationRecord
  belongs_to :user
  validates :user_id, uniqueness: true

  validates :persona, presence: true
  validates :participate_reason, presence: true
  validates :qualified_reason, presence: true
  validates :website, format: URI::DEFAULT_PARSER.make_regexp(%w[http https]), allow_nil: true, allow_blank: true
  validates :interest_reason, presence: true
end
