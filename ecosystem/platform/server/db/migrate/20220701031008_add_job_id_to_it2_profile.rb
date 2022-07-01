# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AddJobIdToIt2Profile < ActiveRecord::Migration[7.0]
  def change
    add_column :it2_profiles, :nhc_job_id, :string, null: true
    add_column :it2_profiles, :nhc_output, :text, null: true
  end
end
