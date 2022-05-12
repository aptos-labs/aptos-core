# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

ActiveAdmin.register User do
  menu priority: 1
  actions :all, except: %i[destroy new]

  permit_params :email, :is_root, :kyc_exempt
  includes :authorizations, :it1_profile

  index do
    selectable_column
    id_column
    column :it1_profile
    column :authorizations
    column 'External ID', :external_id
    column :email
    column :is_root
    column :kyc_exempt
    column :current_sign_in_ip
    column 'Last Sign In', :current_sign_in_at
    column :sign_in_count
    column :created_at
    actions
  end

  filter :email
  filter :external_id
  filter :sign_in_count
  filter :is_root
  filter :kyc_exempt
  filter :current_sign_in_ip
  filter :last_sign_in_ip
  filter :created_at
  filter :updated_at

  show do
    default_main_content do
      row :authorizations
      row :it1_profile
    end
  end

  form do |f|
    f.inputs do
      f.input :email
      f.input :is_root
      f.input :kyc_exempt
    end
    f.actions
  end
end
