# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
class AddSelectedToIt1Profile < ActiveRecord::Migration[7.0]
  def change
    add_column :it1_profiles, :selected, :boolean, default: false, null: false,
                                                   comment: 'Whether this node is selected for participation in IT1.'
  end
end
