# frozen_string_literal: true

class AddAdminFlagToUsers < ActiveRecord::Migration[6.1]
  def change
    change_table :users do |t|
      t.boolean :is_root, null: false, default: false
    end
  end
end
