# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

ActiveAdmin.register Authorization do
  menu priority: 3
  actions :all, except: %i[new]

  permit_params :user_id
  includes :user

  index do
    selectable_column
    id_column

    column :user
    column 'User ID', :user_id
    column :provider
    column :uid
    column :email
    column :username
    column :full_name
    column(:profile_url) { |c| link_to 'view⇗', c.profile_url, target: '_blank' if c.profile_url.present? }
    column :created_at
    column :updated_at

    actions
  end

  filter :user_id, label: 'User ID'
  filter :provider
  filter :uid
  filter :email
  filter :username
  filter :full_name
  filter :created_at
  filter :updated_at

  show do
    default_main_content do
      row :user
    end
  end

  form do |f|
    f.semantic_errors
    f.inputs do
      f.input :user_id, label: 'User ID'
      f.input :provider, input_html: { readonly: true, disabled: true }
      f.input :email, input_html: { readonly: true, disabled: true }
      f.input :username, input_html: { readonly: true, disabled: true }
      f.input :full_name, input_html: { readonly: true, disabled: true }
    end
    f.actions
  end
end
