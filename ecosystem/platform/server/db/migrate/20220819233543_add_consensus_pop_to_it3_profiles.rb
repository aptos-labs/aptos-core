# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AddConsensusPopToIt3Profiles < ActiveRecord::Migration[7.0]
  def change
    add_column :it3_profiles, :consensus_pop, :string
  end
end
