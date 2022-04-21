# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class CreateAuthorizations < ActiveRecord::Migration[6.1]
  def change
    create_table :authorizations do |t|
      t.integer :user_id
      t.string :provider
      t.string :uid
      t.string :email

      t.string :username
      t.string :full_name
      t.text :profile_url

      t.string :token
      t.string :secret
      t.string :refresh_token
      t.boolean :expires
      t.datetime :expires_at
      t.timestamps

      t.index %i[provider uid], name: :index_authorizations_on_provider_and_uid
      t.index [:provider], name: :index_authorizations_on_provider
      t.index [:uid], name: :index_authorizations_on_uid
      t.index [:user_id], name: :index_authorizations_on_user_id
    end
  end
end
