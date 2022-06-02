# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AddFinalChecksForIt1 < ActiveRecord::Migration[7.0]
  def change
    add_column :it1_profiles, :validator_verified_final, :boolean
    add_column :it1_profiles, :metrics_data, :jsonb
  end
end
