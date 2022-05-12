# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
class AddKYCFieldsToUsers < ActiveRecord::Migration[7.0]
  def change
    change_table :users do |t|
      t.boolean :kyc_exempt, default: false
      t.string :completed_persona_inquiry_id
    end
  end
end
