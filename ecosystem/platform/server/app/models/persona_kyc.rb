# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class PersonaKYC < ApplicationRecord
  belongs_to :user
end
