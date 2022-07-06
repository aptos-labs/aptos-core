# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

ActiveAdmin.register It2Survey do
  menu priority: 2
  actions :all, except: %i[destroy edit new]

  permit_params :consensus_key
  includes :user

  index do
    selectable_column
    id_column
    column :user
    column :persona
    column :participate_reason
    column :qualified_reason
    column :website
    column :interest_reason
    column :created_at
    column :updated_at
    actions
  end

  filter :user_id
  filter :persona
  filter :participate_reason
  filter :qualified_reason
  filter :website
  filter :interest_reason
  filter :created_at
  filter :updated_at

  show do
    default_main_content do
      row(:it2_profile) { |survey| survey.user.it2_profile }
    end
  end
end
