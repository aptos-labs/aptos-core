# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class Notification < ApplicationRecord
  include Noticed::Model
  belongs_to :recipient, polymorphic: true
end
