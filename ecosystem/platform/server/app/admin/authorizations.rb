# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

ActiveAdmin.register Authorization do
  menu priority: 3
  actions :all, except: %i[destroy edit new]

  permit_params :consensus_key
  includes :user

  index do
    selectable_column
    id_column

    column :user
    column :provider
    column :uid
    column :email
    column :username
    column :full_name
    column(:profile_url) { |c| link_to 'view⇗', c.profile_url, target: '_blank' if c.profile_url.present? }
    column :expires
    column :expires_at
    column :created_at
    column :updated_at

    actions
  end

  filter :provider
  filter :uid
  filter :email
  filter :username
  filter :full_name
  filter :expires
  filter :expires_at
  filter :created_at
  filter :updated_at

  show do
    default_main_content do
      row :user
    end
  end
end
