# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
class AddTermsToIt1 < ActiveRecord::Migration[7.0]
  def change
    change_table :it1_profiles do |t|
      t.boolean :terms_accepted, default: false
    end
  end
end
