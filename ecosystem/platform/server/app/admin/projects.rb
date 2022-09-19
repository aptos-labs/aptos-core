# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

ActiveAdmin.register Project do
  actions :all

  permit_params :verified
  includes :user

  index do
    selectable_column
    id_column
    column :user
    column :title
    column :short_description
    column :website_url
    column :verified
    actions
  end

  filter :title
  filter :short_description
  filter :website_url
  filter :verified
end
